use crate::command_line_arguments::{BranchCommand, Command, CommandLineArguments};
use crate::error::{Error, Result};
use crate::target_expression::{BranchOrVersion, VersionOrUncommitted};
use biver_configuration::GlobalConfiguration;
use biver_core::data::{BranchName, Snapshot};
use biver_core::repository;
use biver_core::repository::{
    AmendResult, CheckOutBranchResult, CheckOutVersionResult, CommitResult, CreateBranchResult,
    InitializeResult, RebaseResult, RemoveBranchResult, RenameBranchResult, RestoreVersionResult,
    Session, SoftResetResult, UpdateDescriptionResult,
};
use biver_std_environment::StdEnvironment;
use clap::Parser;
use colored::Colorize;
use std::ffi::{OsStr, OsString};
use std::io;
use std::io::{Read, Seek};
use std::process::ExitCode;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

mod command_line_arguments;
mod error;
mod formatting;
mod snapshot_expression;
mod target_expression;
mod viewer;

fn main() -> ExitCode {
    let arguments = CommandLineArguments::parse();

    match run_command(arguments.command) {
        Ok(()) => ExitCode::SUCCESS,

        Err(e) => {
            eprintln!("{}", e.to_string().red());
            ExitCode::FAILURE
        }
    }
}

fn run_command(command: Command) -> Result<()> {
    init_tracing()?;

    match command {
        Command::Status {
            versioned_file_path,
            snapshot,
            all,
        } => {
            let snapshot = snapshot_expression::resolve_optional(snapshot.as_deref())?;
            let session_result =
                repository::start_session(StdEnvironment, &versioned_file_path, snapshot);

            match session_result {
                Ok(session) => {
                    formatting::print_repository_data(
                        session.tree(),
                        session.has_uncommitted_changes()?,
                        all,
                    );
                    Ok(())
                }
                Err(biver_core::Error::RepositoryNotInitialized) => {
                    println!("Not initialized");
                    Ok(())
                }
                Err(e) => Err(Error::from(e)),
            }
        }

        Command::Preview {
            versioned_file_path,
            snapshot,
            target,
        } => {
            let snapshot = snapshot_expression::resolve_optional(snapshot.as_deref())?;
            let session =
                repository::start_session(StdEnvironment, &versioned_file_path, snapshot)?;

            let (_, preview) = preview(&session, Some(&target))?;

            viewer::show_preview(preview)?;

            Ok(())
        }

        Command::Compare {
            versioned_file_path,
            snapshot,
            target1,
            target2,
        } => {
            let snapshot = snapshot_expression::resolve_optional(snapshot.as_deref())?;
            let session =
                repository::start_session(StdEnvironment, &versioned_file_path, snapshot)?;

            let (description1, preview1) = preview(&session, Some(&target1))?;
            let (description2, preview2) = preview(&session, target2.as_deref())?;

            viewer::show_comparison(preview1, description1, preview2, description2)?;

            print_ok();
            Ok(())
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
                        return Err(Error::from_string("Invalid branch name"));
                    };
                    Some(branch_name)
                }
            };

            let config = core_config(versioned_file_path.extension())?;

            let result = repository::initialize(
                &StdEnvironment,
                config,
                &versioned_file_path,
                branch_name,
                description.unwrap_or_default(),
            )?;

            match result {
                InitializeResult::Ok => print_ok(),
                InitializeResult::AlreadyInitialized => print_warn("Already initialized"),
            }

            Ok(())
        }

        Command::Commit {
            versioned_file_path,
            description,
        } => {
            let config = global_config()?;

            let mut session =
                repository::start_session(StdEnvironment, &versioned_file_path, Snapshot::Main)?;

            let result = session.commit(description.unwrap_or_default())?;
            match result {
                CommitResult::Ok(_) => {
                    maybe_auto_tidy_up(&config, &session)?;
                    print_ok();
                    Ok(())
                }
                CommitResult::NoUncommittedChanges => {
                    print_warn("No uncommitted changes");
                    Ok(())
                }
                CommitResult::HeadMustBeOnBranch => {
                    Err(Error::from_string("Head must point at a branch"))
                }
            }
        }

        Command::Amend {
            versioned_file_path,
            confirmed,
            description,
        } => {
            let config = global_config()?;

            let mut session =
                repository::start_session(StdEnvironment, &versioned_file_path, Snapshot::Main)?;

            if !confirmed {
                println!("Are you sure you want to overwrite the head version? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    print_cancelled();
                    return Ok(());
                }
            }

            let result = session.amend(description)?;

            match result {
                AmendResult::Ok(_) => {
                    maybe_auto_tidy_up(&config, &session)?;
                    print_ok();
                    Ok(())
                }
                AmendResult::NoUncommittedChanges => {
                    print_warn("No uncommitted changes");
                    Ok(())
                }
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
            let mut session =
                repository::start_session(StdEnvironment, &versioned_file_path, Snapshot::Main)?;

            let Some(target_id) = target_expression::resolve_version(&session, &target) else {
                return error("Invalid target");
            };

            let result = session.update_description(target_id, description)?;

            match result {
                UpdateDescriptionResult::Ok => {
                    print_ok();
                    Ok(())
                }
                UpdateDescriptionResult::VersionNotFound => error("Invalid target"),
            }
        }

        Command::Discard {
            versioned_file_path,
            confirmed,
        } => {
            let session =
                repository::start_session(StdEnvironment, &versioned_file_path, Snapshot::Main)?;

            if !session.has_uncommitted_changes()? {
                print_warn("No uncommitted changes");
                return Ok(());
            }

            if !confirmed {
                println!("Are you sure you want to discard uncommitted changes? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    print_cancelled();
                    return Ok(());
                }
            }

            session.discard()?;

            print_ok();
            Ok(())
        }

        Command::SoftReset {
            versioned_file_path,
            confirmed,
            target,
        } => {
            let config = global_config()?;

            let mut session =
                repository::start_session(StdEnvironment, &versioned_file_path, Snapshot::Main)?;

            if !confirmed {
                println!("Are you sure you want to reset? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    print_cancelled();
                    return Ok(());
                }
            }

            let Some(target) = target_expression::resolve_as_bs58(&target) else {
                return error("Invalid target");
            };

            let result = session.soft_reset(target)?;

            match result {
                SoftResetResult::Ok(_) => {
                    maybe_auto_tidy_up(&config, &session)?;
                    print_ok();
                    Ok(())
                }
                SoftResetResult::HeadBranchTipIsAlreadyThere => {
                    print_warn("Head branch is already on the target");
                    Ok(())
                }
                SoftResetResult::HeadMustBeOnBranch => error("Head must be on a branch"),
                SoftResetResult::InvalidStopAtVersionId => error("Invalid reset range"),
            }
        }

        Command::Rebase {
            versioned_file_path,
            confirmed,
            rebased_version,
            new_parent,
        } => {
            let config = global_config()?;

            let mut session =
                repository::start_session(StdEnvironment, &versioned_file_path, Snapshot::Main)?;

            if !confirmed {
                println!("Are you sure you want to rebase the current branch? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    print_cancelled();
                    return Ok(());
                }
            }

            let Some(rebased_version_id) = target_expression::resolve_as_bs58(&rebased_version)
            else {
                return error("Invalid rebased version");
            };

            let Some(new_parent_id) = target_expression::resolve_as_bs58(&new_parent) else {
                return error("Invalid new parent");
            };

            let result = session.rebase(rebased_version_id, new_parent_id)?;

            match result {
                RebaseResult::Ok(_) => {
                    maybe_auto_tidy_up(&config, &session)?;
                    print_ok();
                    Ok(())
                }
                RebaseResult::InvalidRebasedVersion => error("Invalid rebased version"),
                RebaseResult::InvalidNewParentVersion => error("Invalid new parent"),
                RebaseResult::CannotRebaseRoot => error("Cannot rebase root"),
                RebaseResult::RebasedVersionDoesNotBelongToHeadBranch => {
                    error("Rebased version must belong to the head branch")
                }
                RebaseResult::HeadMustBeOnBranch => error("Head must point at a branch"),
            }
        }

        Command::Checkout {
            versioned_file_path,
            target,
        } => {
            let mut session =
                repository::start_session(StdEnvironment, &versioned_file_path, Snapshot::Main)?;

            let Some(target) = target_expression::resolve_branch_or_version(&session, &target)
            else {
                return error("Invalid target");
            };

            match target {
                BranchOrVersion::Branch(branch) => {
                    let result = session.check_out_branch(branch)?;

                    match result {
                        CheckOutBranchResult::Ok => {
                            print_ok();
                            Ok(())
                        }
                        CheckOutBranchResult::BranchNotFound => error("Branch not found"),
                    }
                }
                BranchOrVersion::Version(version) => {
                    let result = session.check_out_version(version)?;

                    match result {
                        CheckOutVersionResult::Ok => {
                            print_ok();
                            Ok(())
                        }
                        CheckOutVersionResult::VersionNotFound => error("Version not found"),
                    }
                }
            }
        }

        Command::Restore {
            versioned_file_path,
            snapshot,
            output,
            target,
        } => {
            let snapshot = snapshot_expression::resolve_optional(snapshot.as_deref())?;
            let output = output.unwrap_or_else(|| versioned_file_path.clone());

            let session =
                repository::start_session(StdEnvironment, &versioned_file_path, snapshot)?;

            let Some(target) = target_expression::resolve_version(&session, &target) else {
                return error("Invalid target");
            };

            let result = session.restore_version(target, &output)?;

            match result {
                RestoreVersionResult::Ok => {
                    print_ok();
                    Ok(())
                }
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

                let mut session = repository::start_session(
                    StdEnvironment,
                    &versioned_file_path,
                    Snapshot::Main,
                )?;

                let head_version_d = session.tree().head_version_id();

                let result = session.create_branch(branch_name, head_version_d)?;

                match result {
                    CreateBranchResult::Ok => {
                        print_ok();
                        Ok(())
                    }
                    CreateBranchResult::AlreadyExists => error("Branch already exists"),
                    CreateBranchResult::InvalidBranchTipId => {
                        error("Invalid branch off version ID")
                    }
                }
            }

            BranchCommand::List {
                versioned_file_path,
                snapshot,
            } => {
                let snapshot = snapshot_expression::resolve_optional(snapshot.as_deref())?;
                let session =
                    repository::start_session(StdEnvironment, &versioned_file_path, snapshot)?;
                formatting::print_branch_list(session.tree());
                Ok(())
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

                let mut session = repository::start_session(
                    StdEnvironment,
                    &versioned_file_path,
                    Snapshot::Main,
                )?;

                let result = session.rename_branch(&old_name, new_name)?;

                match result {
                    RenameBranchResult::Ok => {
                        print_ok();
                        Ok(())
                    }
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

                let config = global_config()?;

                let mut session = repository::start_session(
                    StdEnvironment,
                    &versioned_file_path,
                    Snapshot::Main,
                )?;

                if !confirmed {
                    println!("Are you sure you want to remove this branch? (y/N)");
                    let confirmed = read_yes_no_input()?.unwrap_or(false);
                    if !confirmed {
                        print_cancelled();
                        return Ok(());
                    }
                }

                let result = session.remove_branch(&branch_name)?;

                match result {
                    RemoveBranchResult::Ok(_) => {
                        maybe_auto_tidy_up(&config, &session)?;
                        print_ok();
                        Ok(())
                    }
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

        Command::TidyUp {
            versioned_file_path,
        } => {
            let session =
                repository::start_session(StdEnvironment, &versioned_file_path, Snapshot::Main)?;

            session.tidy_up()?;

            Ok(())
        }
    }
}

fn print_ok() {
    println!("{}", "OK".green());
}

fn print_cancelled() {
    println!("{}", "CANCELLED".green());
}

fn print_warn(str: impl AsRef<str>) {
    println!("{}", str.as_ref().yellow());
}

fn error<T>(str: impl Into<String>) -> Result<T> {
    Err(Error::from_string(str))
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

fn global_config() -> Result<GlobalConfiguration> {
    Ok(biver_configuration::read_global()?)
}

fn core_config(versioned_file_extension: Option<&OsStr>) -> Result<biver_core::Config> {
    let global_config = global_config()?;

    let default = global_config.default_file_type_rules;

    let file_rule = if let Some(versioned_file_extension) = versioned_file_extension {
        global_config.file_type_rules.into_iter().find(|fr| {
            fr.extensions
                .iter()
                .any(|e| OsString::from(e) == versioned_file_extension)
        })
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

fn init_tracing() -> Result<()> {
    let rolling_file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_suffix("log")
        .max_log_files(14)
        .build(biver_directories::logs()?)?;

    let file_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_file(false)
        .with_ansi(false)
        .with_writer(rolling_file_appender)
        .with_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,biver=info")),
        );

    tracing_subscriber::registry().with(file_layer).init();

    Ok(())
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

fn should_auto_tidy_up(config: &GlobalConfiguration) -> bool {
    config.auto_tidy_up.unwrap_or(false)
}

fn maybe_auto_tidy_up(config: &GlobalConfiguration, session: &Session) -> Result<()> {
    if should_auto_tidy_up(config) {
        session.tidy_up()?;
    }

    Ok(())
}
