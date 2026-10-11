use crate::command_line_arguments::{BranchCommand, Command, CommandLineArguments};
use crate::error::{Error, Result, Severity, error, warning};
use crate::extensions::OkExt;
use crate::target_expression::BranchOrVersion;
use biver_core::data::BranchName;
use biver_core::repository;
use biver_core::repository::{
    AmendResult, CheckOutBranchResult, CheckOutVersionResult, CommitResult, CreateBranchResult,
    DeleteBranchResult, RenameBranchResult, RestoreVersionResult, SoftResetResult,
    TryStartSessionResult, UpdateDescriptionResult,
};
use clap::Parser;
use colored::Colorize;
use std::io;
use std::io::IsTerminal;
use std::process::ExitCode;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

mod command_line_arguments;
mod error;
mod extensions;
mod formatting;
mod target_expression;
mod viewer;

fn main() -> ExitCode {
    init_tracing();

    let arguments = CommandLineArguments::parse();

    match run_command(arguments.command) {
        Ok(()) => ExitCode::SUCCESS,

        Err(Error {
            message: error_message,
            severity: Severity::Warning,
        }) => {
            println!("{}", error_message.yellow());
            ExitCode::SUCCESS
        }

        Err(Error {
            message: error_message,
            severity: Severity::Error,
        }) => {
            eprintln!("{}", error_message.red());
            ExitCode::FAILURE
        }
    }
}

fn run_command(command: Command) -> Result<()> {
    let config = config()?;

    match command {
        Command::Status {
            versioned_file_path,
            all,
        } => {
            let session_result = repository::try_start_session(config, versioned_file_path)?;

            match session_result {
                TryStartSessionResult::NotInitialized => println!("Not initialized"),
                TryStartSessionResult::Ok(session) => {
                    formatting::print_repository_data(
                        session.tree(),
                        session.has_uncommitted_changes()?,
                        all,
                    );
                }
            }

            success()
        }

        Command::Preview {
            versioned_file_path,
            target,
        } => {
            let session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            let Some(version_id) = target_expression::resolve_version(&session, &target) else {
                return error("Invalid target");
            };

            let Some(preview) = session.preview(version_id)? else {
                return error("No preview available");
            };

            viewer::show_preview(preview)?;

            Ok(())
        }

        Command::Compare {
            versioned_file_path,
            target1,
            target2,
        } => {
            let session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            let Some(version1) = target_expression::resolve_version(&session, &target1)
                .and_then(|id| session.tree().version(id))
            else {
                return error(format!("Invalid target {}", &target1));
            };

            let version2 = match &target2 {
                None => session.tree().head_version(), // TODO by default compare with uncommitted
                Some(target2) => {
                    let Some(version) = target_expression::resolve_version(&session, target2)
                        .and_then(|id| session.tree().version(id))
                    else {
                        return error(format!("Invalid target {}", &target2));
                    };
                    version
                }
            };

            let Some(preview1) = session.preview(version1.id)? else {
                return error(format!("No preview available for {}", version1.id.bs58()));
            };

            let Some(preview2) = session.preview(version2.id)? else {
                return error(format!("No preview available for {}", version2.id.bs58()));
            };

            let formatted_versions =
                formatting::format_versions(session.tree(), &vec![version1, version2]);
            let description1 = &formatted_versions[0];
            let description2 = &formatted_versions[1];

            viewer::show_comparison(preview1, description1, preview2, description2)?;

            success()
        }

        Command::Init {
            versioned_file_path,
            initial_branch_name: branch_name,
            initial_version_description: description,
        } => {
            let branch_name = match branch_name {
                None => None,
                Some(branch_name) => {
                    let Ok(branch_name) = BranchName::new(branch_name) else {
                        return error("Invalid branch name");
                    };
                    Some(branch_name)
                }
            };

            let init_ok = repository::initialize(
                config,
                versioned_file_path,
                branch_name,
                description.unwrap_or_default(),
            )?;

            if init_ok.was_already_initialized {
                warning("Already initialized")
            } else {
                success_ok()
            }
        }

        Command::Commit {
            versioned_file_path,
            description,
        } => {
            let mut session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            let result = session.commit(description.unwrap_or_default())?;
            match result {
                CommitResult::Ok => success_ok(),
                CommitResult::NoUncommittedChanges => warning("No uncommitted changes"),
                CommitResult::HeadMustBeOnBranch => error("Head must point at a branch"),
            }
        }

        Command::Amend {
            versioned_file_path,
            confirmed,
            description,
        } => {
            fail_with_untested()?;

            let mut session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            if !confirmed {
                println!("Are you sure you want to overwrite the head version? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    return success();
                }
            }

            let result = session.amend(description)?;

            match result {
                AmendResult::Ok(_) => success_ok(),
                AmendResult::NoUncommittedChanges => warning("No uncommitted changes"),
                AmendResult::HeadMustBeBranch => error("Head must point at a branch"),
                AmendResult::CannotAmendVersionBelongingToAnotherBranch => {
                    error("Cannot amend head version because it also belongs to other branches")
                }
            }
        }

        Command::UpdateDescription {
            versioned_file_path,
            target,
            description,
        } => {
            let mut session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            let Some(target_id) = target_expression::resolve_version(&session, &target) else {
                return error("Invalid target");
            };

            let result = session.update_description(target_id, description)?;

            match result {
                UpdateDescriptionResult::Ok => success_ok(),
                UpdateDescriptionResult::VersionNotFound => error("Invalid target"),
            }
        }

        Command::Discard {
            versioned_file_path,
            confirmed,
        } => {
            let session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            if !session.has_uncommitted_changes()? {
                return warning("No uncommitted changes");
            }

            if !confirmed {
                println!("Are you sure you want to discard uncommitted changes? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    return success();
                }
            }

            session.discard()?;

            success_ok()
        }

        Command::SoftReset {
            versioned_file_path,
            confirmed,
            target,
        } => {
            fail_with_untested()?;

            let mut session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            if !confirmed {
                println!("Are you sure you want to reset? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    return success();
                }
            }

            let Some(target) = target_expression::resolve_as_bs58(&target) else {
                return error("Invalid target");
            };

            let result = session.soft_reset(target)?;

            match result {
                SoftResetResult::Ok(_) => success_ok(),
                SoftResetResult::HeadMustBeBranch => error("Head must be on a branch"),
                SoftResetResult::InvalidResetRange => error("Invalid reset range"),
                SoftResetResult::ResetRangeOverlapsWithOtherBranches => error(
                    "Cannot reset because some of the erased versions belong to other branches",
                ),
            }
        }

        Command::Checkout {
            versioned_file_path,
            target,
        } => {
            let mut session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            let Some(target) = target_expression::resolve_branch_or_version(&session, &target)
            else {
                return error("Invalid target");
            };

            match target {
                BranchOrVersion::Branch(branch) => {
                    let result = session.check_out_branch(branch)?;

                    match result {
                        CheckOutBranchResult::Ok => success_ok(),
                        CheckOutBranchResult::HasUncommittedChanges => {
                            error("Can not checkout because there are uncommitted changes")
                        }
                        CheckOutBranchResult::BranchNotFound => error("Branch not found"),
                    }
                }
                BranchOrVersion::Version(version) => {
                    let result = session.check_out_version(version)?;

                    match result {
                        CheckOutVersionResult::Ok => success_ok(),
                        CheckOutVersionResult::HasUncommittedChanges => {
                            error("Can not checkout because there are uncommitted changes")
                        }
                        CheckOutVersionResult::VersionNotFound => error("Version not found"),
                    }
                }
            }
        }

        Command::Restore {
            versioned_file_path,
            output,
            target,
        } => {
            fail_with_untested()?;

            let output = output.unwrap_or_else(|| versioned_file_path.clone());

            let session = repository::try_start_session(config, versioned_file_path)?.ok()?;

            let Some(target) = target_expression::resolve_version(&session, &target) else {
                return error("Invalid target");
            };

            let result = session.restore_version(target, &output)?;

            match result {
                RestoreVersionResult::Ok => success_ok(),
                RestoreVersionResult::WouldOverwriteUncommittedChanges => error(
                    "Cannot restore to the versioned file because there are uncommitted changes",
                ),
                RestoreVersionResult::VersionNotFound => error("Invalid target"),
            }
        }

        Command::Branch(branch_command) => match branch_command {
            BranchCommand::Create {
                versioned_file_path,
                name,
            } => {
                let Ok(branch_name) = BranchName::new(name) else {
                    return error("Invalid branch name");
                };

                let mut session =
                    repository::try_start_session(config, versioned_file_path)?.ok()?;

                let head_version_d = session.tree().head_version_id();

                let result = session.create_branch(branch_name, head_version_d)?;

                match result {
                    CreateBranchResult::Ok => success_ok(),
                    CreateBranchResult::AlreadyExists => error("Branch already exists"),
                    CreateBranchResult::InvalidBranchTipId => {
                        error("Invalid branch off version ID")
                    }
                }
            }

            BranchCommand::List {
                versioned_file_path,
            } => {
                let session = repository::try_start_session(config, versioned_file_path)?.ok()?;
                formatting::print_branch_list(session.tree());
                success()
            }

            BranchCommand::Rename {
                versioned_file_path,
                old_name,
                new_name,
            } => {
                let Ok(old_name) = BranchName::new(old_name) else {
                    return error("Invalid old branch name");
                };

                let Ok(new_name) = BranchName::new(new_name) else {
                    return error("Invalid new branch name");
                };

                let mut session =
                    repository::try_start_session(config, versioned_file_path)?.ok()?;

                let result = session.rename_branch(old_name, new_name)?;

                match result {
                    RenameBranchResult::Ok => success_ok(),
                    RenameBranchResult::NewNameAlreadyExists => {
                        error("New name is already used by another branch")
                    }
                    RenameBranchResult::OldBranchNotFound => {
                        error("Old name does not belong to any branch")
                    }
                }
            }

            BranchCommand::Delete {
                versioned_file_path,
                confirmed,
                name,
            } => {
                let Ok(branch_name) = BranchName::new(name) else {
                    return error("Invalid branch name");
                };

                let mut session =
                    repository::try_start_session(config, versioned_file_path)?.ok()?;

                if !confirmed {
                    println!("Are you sure you want to delete this branch? (y/N)");
                    let confirmed = read_yes_no_input()?.unwrap_or(false);
                    if !confirmed {
                        return success();
                    }
                }

                let result = session.delete_branch(branch_name)?;

                match result {
                    DeleteBranchResult::Ok(_) => success_ok(),
                    DeleteBranchResult::BranchNotFound => error("Branch does not exist"),
                    DeleteBranchResult::CannotDeleteHead => {
                        error("Cannot delete the version currently pointed at by head")
                    }
                }
            }
        },
    }
}

fn success_ok() -> Result<()> {
    println!("{}", "OK".green());
    Ok(())
}

fn success() -> Result<()> {
    Ok(())
}

fn read_yes_no_input() -> Result<Option<bool>> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.eq_ignore_ascii_case("y") {
        return Ok(Some(true));
    }

    if input.eq_ignore_ascii_case("n") {
        return Ok(Some(false));
    }

    Ok(None)
}

fn into_core_configuration(value: biver_configuration::Configuration) -> biver_core::Configuration {
    biver_core::Configuration {
        create_patch_command: value.create_patch_command,
        apply_patch_command: value.apply_patch_command,
        file_type_rules: value
            .file_type_rules
            .into_iter()
            .map(into_core_file_type_rule)
            .collect(),
    }
}

fn into_core_file_type_rule(value: biver_configuration::FileTypeRule) -> biver_core::FileTypeRule {
    biver_core::FileTypeRule {
        extensions: value.extensions,
        preview_command: value.preview_command,
    }
}

fn config() -> Result<biver_core::Configuration> {
    Ok(into_core_configuration(biver_configuration::read()?))
}

fn init_tracing() {
    let stdout_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_file(false)
        .with_ansi(io::stdout().is_terminal())
        .with_writer(io::stdout)
        .with_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn,biver=trace")),
        );

    tracing_subscriber::registry().with(stdout_layer).init();
}

fn fail_with_untested() -> Result<()> {
    Err(Error {
        message: "Operation is not tested yet".to_string(),
        severity: Severity::Error,
    })
}
