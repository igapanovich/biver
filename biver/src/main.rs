use crate::command_line_arguments::{BranchCommand, Command, CommandLineArguments};
use crate::error::{Error, Result, Severity, error, warning};
use crate::extensions::ResultExt;
use crate::target_expression::{BranchOrVersion, VersionOrUncommitted};
use biver_core::data::BranchName;
use biver_core::repository;
use biver_core::repository::{
    AmendResult, CheckOutBranchResult, CheckOutVersionResult, CommitResult, CreateBranchResult,
    InitializeResult, RemoveBranchResult, RenameBranchResult, RestoreVersionResult, Session,
    SoftResetResult, TryStartSessionResult, UpdateDescriptionResult,
};
use clap::Parser;
use colored::Colorize;
use std::ffi::{OsStr, OsString};
use std::io;
use std::io::{IsTerminal, Read, Seek};
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
    match command {
        Command::Status {
            versioned_file_path,
            all,
        } => {
            let session_result = repository::try_start_session(&versioned_file_path)?;

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
            let session = repository::try_start_session(&versioned_file_path)?.result()?;

            let (_, preview) = preview(&session, Some(&target))?;

            viewer::show_preview(preview)?;

            Ok(())
        }

        Command::Compare {
            versioned_file_path,
            target1,
            target2,
        } => {
            let session = repository::try_start_session(&versioned_file_path)?.result()?;

            let (description1, preview1) = preview(&session, Some(&target1))?;
            let (description2, preview2) = preview(&session, target2.as_deref())?;

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

            let config = core_config(versioned_file_path.extension())?;

            let init_result = repository::initialize(
                config,
                &versioned_file_path,
                branch_name,
                description.unwrap_or_default(),
            )?;

            match init_result {
                InitializeResult::Ok => success_ok(),
                InitializeResult::AlreadyInitialized => warning("Already initialized"),
            }
        }

        Command::Commit {
            versioned_file_path,
            description,
        } => {
            let mut session = repository::try_start_session(&versioned_file_path)?.result()?;

            let result = session.commit(description.unwrap_or_default())?;
            match result {
                CommitResult::Ok(_) => success_ok(),
                CommitResult::NoUncommittedChanges => warning("No uncommitted changes"),
                CommitResult::HeadMustBeOnBranch => error("Head must point at a branch"),
            }
        }

        Command::Amend {
            versioned_file_path,
            confirmed,
            description,
        } => {
            let mut session = repository::try_start_session(&versioned_file_path)?.result()?;

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
                AmendResult::HeadMustBeOnBranch => error("Head must point at a branch"),
                AmendResult::CannotAmendRootWhenOtherBranchesExist => {
                    error("Cannot amend root when there is more than one branch")
                }
            }
        }

        Command::UpdateDescription {
            versioned_file_path,
            target,
            description,
        } => {
            let mut session = repository::try_start_session(&versioned_file_path)?.result()?;

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
            let session = repository::try_start_session(&versioned_file_path)?.result()?;

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
            let mut session = repository::try_start_session(&versioned_file_path)?.result()?;

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
                SoftResetResult::HeadBranchTipIsAlreadyThere => {
                    warning("Head branch is already on the target")
                }
                SoftResetResult::HeadMustBeOnBranch => error("Head must be on a branch"),
                SoftResetResult::InvalidStopAtVersionId => error("Invalid reset range"),
            }
        }

        Command::Checkout {
            versioned_file_path,
            target,
        } => {
            let mut session = repository::try_start_session(&versioned_file_path)?.result()?;

            let Some(target) = target_expression::resolve_branch_or_version(&session, &target)
            else {
                return error("Invalid target");
            };

            match target {
                BranchOrVersion::Branch(branch) => {
                    let result = session.check_out_branch(branch)?;

                    match result {
                        CheckOutBranchResult::Ok => success_ok(),
                        CheckOutBranchResult::BranchNotFound => error("Branch not found"),
                    }
                }
                BranchOrVersion::Version(version) => {
                    let result = session.check_out_version(version)?;

                    match result {
                        CheckOutVersionResult::Ok => success_ok(),
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
            let output = output.unwrap_or_else(|| versioned_file_path.clone());

            let session = repository::try_start_session(&versioned_file_path)?.result()?;

            let Some(target) = target_expression::resolve_version(&session, &target) else {
                return error("Invalid target");
            };

            let result = session.restore_version(target, &output)?;

            match result {
                RestoreVersionResult::Ok => success_ok(),
                RestoreVersionResult::WouldOverwriteUncommittedChanges => error(
                    "Cannot restore into the versioned file because there are uncommitted changes",
                ),
                RestoreVersionResult::VersionNotFound => error("Invalid target"),
                RestoreVersionResult::CannotRestoreIntoRepositoryDir => {
                    error("Cannot restore into a path inside the repository")
                }
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

                let mut session = repository::try_start_session(&versioned_file_path)?.result()?;

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
                let session = repository::try_start_session(&versioned_file_path)?.result()?;
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

                let mut session = repository::try_start_session(&versioned_file_path)?.result()?;

                let result = session.rename_branch(&old_name, new_name)?;

                match result {
                    RenameBranchResult::Ok => success_ok(),
                    RenameBranchResult::NewNameAlreadyExists => {
                        error("New name is already used by another branch")
                    }
                    RenameBranchResult::OldNameNotFound => {
                        error("Old name does not belong to any branch")
                    }
                }
            }

            BranchCommand::Remove {
                versioned_file_path,
                confirmed,
                name,
            } => {
                let Ok(branch_name) = BranchName::new(name) else {
                    return error("Invalid branch name");
                };

                let mut session = repository::try_start_session(&versioned_file_path)?.result()?;

                if !confirmed {
                    println!("Are you sure you want to remove this branch? (y/N)");
                    let confirmed = read_yes_no_input()?.unwrap_or(false);
                    if !confirmed {
                        return success();
                    }
                }

                let result = session.remove_branch(&branch_name)?;

                match result {
                    RemoveBranchResult::Ok(_) => success_ok(),
                    RemoveBranchResult::BranchNotFound => error("Branch does not exist"),
                    RemoveBranchResult::CannotRemoveHeadBranch => {
                        error("Cannot remove the branch currently pointed at by head")
                    }
                    RemoveBranchResult::CannotRemoveHeadVersion => {
                        error("Cannot remove the version currently pointed at by head")
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

fn core_config(versioned_file_extension: Option<&OsStr>) -> Result<biver_core::Config> {
    let global_config = biver_configuration::read_global()?;

    let default = global_config.default_file_type_rules;

    let file_rule = if let Some(versioned_file_extension) = versioned_file_extension {
        global_config
            .file_type_rules
            .into_iter()
            .filter(|fr| {
                fr.extensions
                    .iter()
                    .any(|e| OsString::from(e) == versioned_file_extension)
            })
            .next()
    } else {
        None
    };

    let mut create_patch_command = default.create_patch_command;
    let mut apply_patch_command = default.apply_patch_command;
    let mut preview_command = default.preview_command;

    if let Some(file_rule) = file_rule {
        if let Some(c) = file_rule.create_patch_command {
            create_patch_command = c;
        }

        if let Some(c) = file_rule.apply_patch_command {
            apply_patch_command = c;
        }

        if let Some(c) = file_rule.preview_command {
            preview_command = Some(c);
        }
    }

    let config = biver_core::Config {
        create_patch_command,
        apply_patch_command,
        preview_command,
    };

    Ok(config)
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

fn preview<'a>(
    session: &'a Session,
    target_expr: Option<&'a str>,
) -> Result<(&'a str, impl Read + Seek)> {
    let target_expr = target_expr.unwrap_or(target_expression::UNCOMMITTED_CHANGES_EXPRESSION);

    let Some(target) = target_expression::resolve_version_or_uncommitted(session, target_expr)
    else {
        return error(format!("Invalid target {}", target_expr));
    };

    let preview = match target {
        VersionOrUncommitted::Version(version_id) => session.preview(version_id)?,
        VersionOrUncommitted::Uncommitted => session.preview_versioned_file()?,
    };

    let Some(preview) = preview else {
        return error(format!("No preview available for {}", target_expr));
    };

    Ok((target_expr, preview))
}
