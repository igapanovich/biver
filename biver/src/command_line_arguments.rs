use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
pub struct CommandLineArguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show the current status of the repository
    #[command(alias = "st")]
    Status {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Show all versions (by default, limited to 20 most recent)
        #[arg(short = 'a', long = "all")]
        all: bool,
    },

    /// Preview a version
    #[command(alias = "pv")]
    Preview {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Target branch or version to preview. May be one of the following (in order of precedence): branch name, version ID, head offset (@, @1, @2), version nickname (adjective-noun, adjectivenoun, an).
        target: String,
    },

    /// Compare two versions using their previews
    #[command(alias = "cmp")]
    Compare {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Target branch or version to compare. May be one of the following (in order of precedence): branch name, version ID, head offset (@, @1, @2), version nickname (adjective-noun, adjectivenoun, an).
        target1: String,

        /// (Default: head) Target branch or version to compare. May be one of the following (in order of precedence): branch name, version ID, head offset (@, @1, @2), version nickname (adjective-noun, adjectivenoun, an).
        target2: Option<String>,
    },

    /// Initialize a new repository
    Init {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// The initial branch name (default: main)
        #[arg(short = 'b', long = "branch")]
        initial_branch_name: Option<String>,

        /// Description of the initial version
        #[arg(value_name = "INITIAL_VERSION_DESCRIPTION")]
        initial_version_description: Option<String>,
    },

    /// Commit current changes to a new version
    #[command(alias = "ct")]
    Commit {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Description of the new version
        #[arg(value_name = "DESCRIPTION")]
        description: Option<String>,
    },

    /// Amend the head version
    Amend {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Do not ask for confirmation
        #[arg(short = 'y', long = "yes")]
        confirmed: bool,

        /// New description
        #[arg(value_name = "DESCRIPTION")]
        description: Option<String>,
    },

    /// Update description of the specified version
    UpdateDescription {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Target version to reword. Must be a version ID.
        target: String,

        /// New description
        #[arg(value_name = "DESCRIPTION")]
        description: String,
    },

    /// Discard uncommitted changes
    Discard {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Do not ask for confirmation
        #[arg(short = 'y', long = "yes")]
        confirmed: bool,
    },

    /// Reset the current branch back to the specified version
    SoftReset {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Do not ask for confirmation
        #[arg(short = 'y', long = "yes")]
        confirmed: bool,

        /// Target version to reset to. Must be a version ID.
        target: String,
    },

    /// Check out a specific branch or version
    Checkout {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Target branch or version to preview. May be one of the following (in order of precedence): branch name, version ID, head offset (@, @1, @2), version nickname (adjective-noun, adjectivenoun, an).
        target: String,
    },

    /// Set the versioned file to the state it was in when the specified version was created
    Restore {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Output file path. If not specified, the versioned file path will be used.
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,

        /// Target branch or version to restore. May be one of the following (in order of precedence): branch name, version ID, head offset (@, @1, @2), version nickname (adjective-noun, adjectivenoun, an).
        target: String,
    },

    /// Branch commands
    #[command(subcommand)]
    Branch(BranchCommand),
}

#[derive(Subcommand)]
pub enum BranchCommand {
    /// Create branch
    Create {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// The name of the new branch
        name: String,
    },

    /// List branches
    List {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,
    },

    /// Rename a branch
    Rename {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        #[arg(value_name = "OLD_NAME")]
        old_name: String,

        #[arg(value_name = "NEW_NAME")]
        new_name: String,
    },

    /// Remove a branch
    Remove {
        #[arg(short = 'f', long = "file", env = "BIVER_VERSIONED_FILE")]
        versioned_file_path: PathBuf,

        /// Do not ask for confirmation
        #[arg(short = 'y', long = "yes")]
        confirmed: bool,

        #[arg(value_name = "NAME")]
        name: String,
    },
}
