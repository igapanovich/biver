use crate::command_line_arguments::{
    Command, CommandLineArguments, CreateCommand, DeleteCommand, ListCommand, RenameCommand,
};
use crate::error::{Error, Result, Severity, error, warning};
use biver_core::data::Repository;
use biver_core::{RepositoryPaths, operations as ops};
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
mod formatting;
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
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let repo = ops::read_repository(&paths)?;

            match repo {
                ops::read_repository::Outcome::NotInitialized => println!("Not initialized"),
                ops::read_repository::Outcome::Initialized(repository_data) => {
                    let has_uncommitted_changes =
                        ops::has_uncommitted_changes(&paths, &repository_data)?;
                    formatting::print_repository_data(
                        &repository_data,
                        has_uncommitted_changes,
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
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let repo = ops::read_repository(&paths)?.initialized()?;

            let version = match ops::resolve_version(&repo, &target) {
                ops::resolve_version::Outcome::InvalidTarget => {
                    return error("Invalid target");
                }
                ops::resolve_version::Outcome::Ok(version) => version,
            };

            let Some(preview_file_path) = paths.preview_path(version) else {
                return error("No preview available");
            };

            viewer::show_preview(&preview_file_path)?;

            Ok(())
        }

        Command::Compare {
            versioned_file_path,
            target1,
            target2,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let repo = ops::read_repository(&paths)?.initialized()?;

            let version_and_preview = |target: Option<&str>| {
                let version = match target {
                    None => repo.head_version(),
                    Some(target) => match ops::resolve_version(&repo, target) {
                        ops::resolve_version::Outcome::InvalidTarget => {
                            return error(format!("Invalid target {}", target));
                        }
                        ops::resolve_version::Outcome::Ok(version) => version,
                    },
                };

                match paths.preview_path(version) {
                    Some(preview) => Ok((version, preview)),
                    None => error(format!("No preview available for {}", version.id.bs58())),
                }
            };

            let (version1, preview_file_path1) = version_and_preview(Some(&target1))?;
            let (version2, preview_file_path2) = version_and_preview(target2.as_deref())?;

            let formatted_versions = formatting::format_versions(&repo, &vec![version1, version2]);
            let description1 = &formatted_versions[0];
            let description2 = &formatted_versions[1];

            viewer::show_comparison(
                &preview_file_path1,
                description1,
                &preview_file_path2,
                description2,
            )?;

            success()
        }

        Command::Init {
            versioned_file_path,
            initial_branch_name: branch_name,
            initial_version_description: description,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let config = config()?;

            let result = ops::init(
                &config,
                &paths,
                branch_name.as_deref(),
                description.as_deref(),
            )?;

            match result {
                ops::init::Outcome::Ok => success_ok(),
                ops::init::Outcome::AlreadyInitialized => warning("Already initialized"),
                ops::init::Outcome::InvalidBranchName => error("Invalid branch name"),
            }
        }

        Command::Commit {
            versioned_file_path,
            description,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let mut repo = ops::read_repository(&paths)?.initialized()?;
            let config = config()?;

            let result = ops::commit(&config, &paths, &mut repo, description.as_deref())?;

            match result {
                ops::commit::Outcome::Ok => success_ok(),
                ops::commit::Outcome::NothingToCommit => warning("Nothing to commit"),
                ops::commit::Outcome::HeadMustBeOnBranch => error("Head must be on a branch"),
            }
        }

        Command::Amend {
            versioned_file_path,
            confirmed,
            description,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let mut repo = ops::read_repository(&paths)?.initialized()?;
            let config = config()?;

            if !confirmed {
                println!("Are you sure you want to overwrite the head version? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    return success();
                }
            }

            let result = ops::amend(&config, &paths, &mut repo, description.as_deref())?;

            match result {
                ops::amend::Outcome::Ok => success_ok(),
                ops::amend::Outcome::NoUncommittedChanges => warning("No uncommitted changes"),
                ops::amend::Outcome::HeadMustBeBranch => error("Head must be on a branch"),
                ops::amend::Outcome::CannotAmendParent => {
                    error("Cannot amend head version because it has children")
                }
                ops::amend::Outcome::HeadEqualsParent => error(
                    "Amend would result in head having identical content to its parent's. Use hard reset instead.",
                ),
            }
        }

        Command::Reword {
            versioned_file_path,
            target,
            description,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let mut repo = ops::read_repository(&paths)?.initialized()?;

            let result = ops::reword(&paths, &mut repo, &target, &description)?;

            match result {
                ops::reword::Outcome::Ok => success_ok(),
                ops::reword::Outcome::InvalidTarget => error("Invalid target"),
            }
        }

        Command::Discard {
            versioned_file_path,
            confirmed,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let repo = ops::read_repository(&paths)?.initialized()?;
            let config = config()?;

            if !ops::has_uncommitted_changes(&paths, &repo)? {
                return warning("No uncommitted changes");
            }

            if !confirmed {
                println!("Are you sure you want to discard uncommitted changes? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    return success();
                }
            }

            ops::discard(&config, &paths, &repo)?;

            success_ok()
        }

        Command::Reset {
            versioned_file_path,
            hard,
            confirmed,
            target,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let mut repo = ops::read_repository(&paths)?.initialized()?;
            let config = config()?;

            if !confirmed {
                println!("Are you sure you want to reset? (y/N)");
                let confirmed = read_yes_no_input()?.unwrap_or(false);
                if !confirmed {
                    return success();
                }
            }

            let result = ops::reset(&paths, &mut repo, target.as_str())?;

            match result {
                ops::reset::Outcome::Ok => {
                    if hard {
                        ops::discard(&config, &paths, &repo)?;
                    }

                    success_ok()
                }
                ops::reset::Outcome::HeadMustBeBranch => error("Head must be on a branch"),
                ops::reset::Outcome::InvalidTarget => error("Invalid target"),
                ops::reset::Outcome::CannotLeaveOrphans => error(
                    "Reset would leave orphaned versions. Make sure none of the erased versions have children outside of the reset range.",
                ),
            }
        }

        Command::Checkout {
            versioned_file_path,
            target,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let mut repo = ops::read_repository(&paths)?.initialized()?;
            let config = config()?;

            let result = ops::check_out(&config, &paths, &mut repo, &target)?;

            match result {
                ops::check_out::Outcome::Ok => success_ok(),
                ops::check_out::Outcome::InvalidTarget => error("Invalid target"),
            }
        }

        Command::Restore {
            versioned_file_path,
            output,
            target,
        } => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let repo = ops::read_repository(&paths)?.initialized()?;
            let config = config()?;

            let result = ops::restore(&config, &paths, &repo, &target, output.as_deref())?;

            match result {
                ops::restore::Outcome::Ok => success_ok(),
                ops::restore::Outcome::BlockedByUncommittedChanges => error(
                    "Cannot restore to the versioned file because there are uncommitted changes",
                ),
                ops::restore::Outcome::InvalidTarget => error("Invalid target"),
            }
        }

        Command::Create(CreateCommand::Branch {
            versioned_file_path,
            checkout,
            name,
        }) => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let mut repo = ops::read_repository(&paths)?.initialized()?;

            let result = ops::branch::create(&paths, &mut repo, &name, checkout)?;

            match result {
                ops::branch::create::Outcome::Ok => success_ok(),
                ops::branch::create::Outcome::BranchAlreadyExists => error("Branch already exists"),
                ops::branch::create::Outcome::InvalidBranchName => error("Invalid branch name"),
            }
        }

        Command::List(ListCommand::Branches {
            versioned_file_path,
        }) => {
            let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
            let repo = ops::read_repository(&paths)?.initialized()?;

            formatting::print_branch_list(&repo);

            success()
        }

        Command::Rename(rename_command) => match rename_command {
            RenameCommand::Branch {
                versioned_file_path,
                old_name,
                new_name,
            } => {
                let paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
                let mut repo = ops::read_repository(&paths)?.initialized()?;

                let result = ops::branch::rename(&paths, &mut repo, &old_name, &new_name)?;

                match result {
                    ops::branch::rename::Outcome::Ok => success_ok(),
                    ops::branch::rename::Outcome::AnotherBranchExistsWithSameName => {
                        error("Another branch exists with the same name")
                    }
                    ops::branch::rename::Outcome::BranchDoesNotExist => {
                        error("Branch does not exist")
                    }
                }
            }
        },

        Command::Delete(delete_command) => match delete_command {
            DeleteCommand::Branch {
                versioned_file_path,
                confirmed,
                name,
            } => {
                let repo_paths = RepositoryPaths::from_versioned_file_path(versioned_file_path);
                let mut repo_data = ops::read_repository(&repo_paths)?.initialized()?;

                if !confirmed {
                    println!("Are you sure you want to delete this branch? (y/N)");
                    let confirmed = read_yes_no_input()?.unwrap_or(false);
                    if !confirmed {
                        return success();
                    }
                }

                let result = ops::branch::delete(&repo_paths, &mut repo_data, &name)?;

                match result {
                    ops::branch::delete::Outcome::Ok => success_ok(),
                    ops::branch::delete::Outcome::BranchDoesNotExist => {
                        error("Branch does not exist")
                    }
                    ops::branch::delete::Outcome::CannotDeleteHead => {
                        error("Cannot delete the version currently pointed at by HEAD")
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

trait RepositoryDataResultExtensions {
    fn initialized(self) -> Result<Repository>;
}

impl RepositoryDataResultExtensions for ops::read_repository::Outcome {
    fn initialized(self) -> Result<Repository> {
        match self {
            ops::read_repository::Outcome::NotInitialized => Err(Error {
                message: "Not initialized".to_string(),
                severity: Severity::Error,
            }),
            ops::read_repository::Outcome::Initialized(repository_data) => Ok(repository_data),
        }
    }
}

fn into_core_configuration(
    value: biver_configuration::Configuration,
) -> biver_core::configuration::Configuration {
    biver_core::configuration::Configuration {
        create_patch_command: value.create_patch_command,
        apply_patch_command: value.apply_patch_command,
        file_type_rules: value
            .file_type_rules
            .into_iter()
            .map(into_core_file_type_rule)
            .collect(),
    }
}

fn into_core_file_type_rule(
    value: biver_configuration::FileTypeRule,
) -> biver_core::configuration::FileTypeRule {
    biver_core::configuration::FileTypeRule {
        extensions: value.extensions,
        preview_command: value.preview_command,
    }
}

fn config() -> Result<biver_core::configuration::Configuration> {
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
