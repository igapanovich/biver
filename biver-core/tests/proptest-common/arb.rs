use crate::helpers::byte_chunk::ByteChunk;
use crate::helpers::file_operation::FileOperation;
use crate::helpers::repository_blueprint::{
    BranchBlueprint, HeadBlueprint, InitialBranchBlueprint, RepositoryBlueprint, VersionBlueprint,
};
use crate::helpers::test_env::TestEnv;
use crate::helpers::time_control_environment::TimeControlEnvironment;
use crate::helpers::version_path::VersionPath;
use biver_core::data::{BranchName, VersionId};
use biver_std_environment::StdEnvironment;
use proptest::collection::vec;
use proptest::option;
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::string::string_regex;
use std::path::PathBuf;

pub fn test_env() -> impl Strategy<Value = TestEnv<StdEnvironment>> {
    repository_blueprint(&[]).prop_map(|b| TestEnv::from_blueprint(StdEnvironment, b))
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TestEnvOption {
    UncommittedChanges,
    NoUncommittedChanges,
    HeadOnBranch,
    HeadOnVersion,
    AtLeastTwoBranches,
    SingleBranch,
    AtLeastTwoVersions,
}

pub fn test_env_with_opts(
    options: &[TestEnvOption],
) -> impl Strategy<Value = TestEnv<StdEnvironment>> {
    repository_blueprint(options).prop_map(|b| TestEnv::from_blueprint(StdEnvironment, b))
}

pub fn test_env_with_time_control() -> impl Strategy<Value = TestEnv<TimeControlEnvironment>> {
    repository_blueprint(&[])
        .prop_map(|b| TestEnv::from_blueprint(TimeControlEnvironment::new(), b))
}

pub fn bytes() -> impl Strategy<Value = ByteChunk> {
    prop_oneof![
        1 => Just(ByteChunk::from(Vec::new())),
        1 => vec(any::<u8>(), 1).prop_map(ByteChunk::from),
        10 => vec(any::<u8>(), 2..=2048).prop_map(ByteChunk::from),
    ]
}

fn non_empty_bytes() -> impl Strategy<Value = ByteChunk> {
    prop_oneof![
        1 => vec(any::<u8>(), 1).prop_map(ByteChunk::from),
        11 => vec(any::<u8>(), 2..=2048).prop_map(ByteChunk::from),
    ]
}

fn file_operation() -> impl Strategy<Value = FileOperation> {
    prop_oneof![
        1 => file_operation_overwrite(),
        24 => file_operation_splice(),
    ]
}

fn file_operation_splice() -> impl Strategy<Value = FileOperation> {
    (any::<Index>(), any::<Index>(), bytes()).prop_map(|(range_start, range_length, bytes)| {
        FileOperation::Splice {
            range_start,
            range_length: Some(range_length),
            bytes,
        }
    })
}

pub fn file_operation_non_empty_insert() -> impl Strategy<Value = FileOperation> {
    (any::<Index>(), non_empty_bytes()).prop_map(|(range_start, bytes)| FileOperation::Splice {
        range_start,
        range_length: None,
        bytes,
    })
}

fn file_operation_overwrite() -> impl Strategy<Value = FileOperation> {
    bytes().prop_map(FileOperation::Overwrite)
}

pub fn version_path() -> impl Strategy<Value = VersionPath> {
    (any::<Index>(), any::<Index>()).prop_map(|(branch, depth_from_tip)| VersionPath {
        branch,
        depth_from_tip,
    })
}

pub fn branch_name() -> impl Strategy<Value = BranchName> {
    string_regex("[A-Za-z0-9_-]+")
        .unwrap()
        .prop_map(|n| BranchName::new(n).unwrap())
}

pub fn version_id() -> impl Strategy<Value = VersionId> {
    any::<u128>().prop_map(VersionId::from_u128)
}

fn repository_blueprint(options: &[TestEnvOption]) -> impl Strategy<Value = RepositoryBlueprint> {
    let head = if options.contains(&TestEnvOption::HeadOnBranch) {
        head_branch_blueprint().boxed()
    } else if options.contains(&TestEnvOption::HeadOnVersion) {
        head_version_blueprint().boxed()
    } else {
        head_blueprint().boxed()
    };

    let uncommitted_changes = if options.contains(&TestEnvOption::NoUncommittedChanges) {
        Just(None).boxed()
    } else if options.contains(&TestEnvOption::UncommittedChanges) {
        file_operation_non_empty_insert()
            .prop_map(|i| Some(vec![i]))
            .boxed()
    } else {
        option::of(vec(file_operation(), 1..=4)).boxed()
    };

    let min_initial_branch_version_count = if options.contains(&TestEnvOption::AtLeastTwoVersions) {
        2
    } else {
        1
    };

    let other_branches = if options.contains(&TestEnvOption::AtLeastTwoBranches) {
        vec(branch_blueprint(), 1..=4).boxed()
    } else if options.contains(&TestEnvOption::SingleBranch) {
        Just(Vec::new()).boxed()
    } else {
        vec(branch_blueprint(), 0..=4).boxed()
    };

    (
        initial_branch_blueprint(min_initial_branch_version_count),
        other_branches,
        head,
        uncommitted_changes,
    )
        .prop_map(
            |(initial_branch, other_branches, head, uncommitted_changes)| RepositoryBlueprint {
                initial_branch,
                other_branches,
                head,
                uncommitted_changes,
            },
        )
}

fn head_blueprint() -> impl Strategy<Value = HeadBlueprint> {
    prop_oneof![
        4 => head_branch_blueprint(),
        1 => head_version_blueprint(),
    ]
}

fn head_branch_blueprint() -> impl Strategy<Value = HeadBlueprint> {
    any::<Index>().prop_map(HeadBlueprint::Branch)
}

fn head_version_blueprint() -> impl Strategy<Value = HeadBlueprint> {
    version_path().prop_map(HeadBlueprint::Version)
}

fn initial_branch_blueprint(
    min_version_count: usize,
) -> impl Strategy<Value = InitialBranchBlueprint> {
    if min_version_count >= 9 {
        panic!("min_version_count should be below 9");
    }

    (
        option::of(branch_name()),
        vec(version_blueprint(), min_version_count..=8),
    )
        .prop_map(|(name, mut versions)| InitialBranchBlueprint {
            name,
            first_version: versions.remove(0),
            following_versions: versions,
        })
}

fn branch_blueprint() -> impl Strategy<Value = BranchBlueprint> {
    (
        branch_name(),
        vec(version_blueprint(), 0..=8),
        any::<Index>(),
    )
        .prop_map(|(name, versions, diverge_at)| BranchBlueprint {
            name,
            versions,
            diverge_at,
        })
}

fn version_blueprint() -> impl Strategy<Value = VersionBlueprint> {
    (vec(file_operation(), 1..=4), any::<String>()).prop_map(|(changes, description)| {
        VersionBlueprint {
            changes,
            description,
        }
    })
}

pub fn fs_path_segment() -> impl Strategy<Value = PathBuf> {
    "[^<>:\"/\\\\|?*\\x00-\\x1f]{1,32}"
        .prop_filter("fs_path/invalid_segment", |s| {
            if [
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9", ".", "..",
            ]
            .contains(&s.as_str())
            {
                return false;
            }

            if s.ends_with(" ") || s.ends_with(".") {
                return false;
            }

            true
        })
        .prop_map(PathBuf::from)
}

pub fn fs_path() -> impl Strategy<Value = PathBuf> {
    vec(fs_path_segment(), 1..=8).prop_map(|mut segments| {
        let mut path = segments.pop().unwrap();

        while !segments.is_empty() {
            path = path.join(segments.pop().unwrap());
        }

        path
    })
}

pub mod repository_operation {
    use crate::helpers::arb;
    use crate::helpers::repository_operation::{
        RepositoryOperation, TargetBranch, TargetFile, TargetVersion,
    };
    use proptest::collection::vec;
    use proptest::option;
    use proptest::prelude::*;
    use proptest::sample::Index;

    pub fn change_versioned_file() -> impl Strategy<Value = RepositoryOperation> {
        arb::file_operation().prop_map(RepositoryOperation::ChangeVersionedFile)
    }

    pub fn commit() -> impl Strategy<Value = RepositoryOperation> {
        any::<String>().prop_map(RepositoryOperation::Commit)
    }

    pub fn amend() -> impl Strategy<Value = RepositoryOperation> {
        option::of(any::<String>()).prop_map(RepositoryOperation::Amend)
    }

    pub fn discard() -> impl Strategy<Value = RepositoryOperation> {
        Just(RepositoryOperation::Discard)
    }

    pub fn check_out_branch() -> impl Strategy<Value = RepositoryOperation> {
        target_branch().prop_map(RepositoryOperation::CheckOutBranch)
    }

    pub fn check_out_version() -> impl Strategy<Value = RepositoryOperation> {
        target_version().prop_map(RepositoryOperation::CheckOutVersion)
    }

    pub fn update_description() -> impl Strategy<Value = RepositoryOperation> {
        (target_version(), any::<String>()).prop_map(|(version, description)| {
            RepositoryOperation::UpdateDescription(version, description)
        })
    }

    pub fn restore_version() -> impl Strategy<Value = RepositoryOperation> {
        (target_version(), target_file())
            .prop_map(|(version, file)| RepositoryOperation::RestoreVersion(version, file))
    }

    pub fn soft_reset() -> impl Strategy<Value = RepositoryOperation> {
        target_version().prop_map(RepositoryOperation::SoftReset)
    }

    pub fn rebase() -> impl Strategy<Value = RepositoryOperation> {
        (target_version(), target_version())
            .prop_map(|(target, new_parent)| RepositoryOperation::Rebase(target, new_parent))
    }

    pub fn create_branch() -> impl Strategy<Value = RepositoryOperation> {
        (arb::branch_name(), target_version())
            .prop_map(|(name, version)| RepositoryOperation::CreateBranch(name, version))
    }

    pub fn rename_branch() -> impl Strategy<Value = RepositoryOperation> {
        (target_branch(), arb::branch_name()).prop_map(|(target_branch, new_name)| {
            RepositoryOperation::RenameBranch(target_branch, new_name)
        })
    }

    pub fn remove_branch() -> impl Strategy<Value = RepositoryOperation> {
        target_branch().prop_map(RepositoryOperation::RemoveBranch)
    }

    pub fn tidy_up() -> impl Strategy<Value = RepositoryOperation> {
        Just(RepositoryOperation::TidyUp)
    }

    pub fn valid_target_branch() -> impl Strategy<Value = TargetBranch> {
        any::<Index>().prop_map(TargetBranch::Valid)
    }

    pub fn any_target_branch() -> impl Strategy<Value = TargetBranch> {
        arb::branch_name().prop_map(TargetBranch::Any)
    }

    pub fn target_branch() -> impl Strategy<Value = TargetBranch> {
        prop_oneof![
            4 => valid_target_branch(),
            1 => any_target_branch(),
        ]
    }

    pub fn valid_target_version() -> impl Strategy<Value = TargetVersion> {
        any::<Index>().prop_map(TargetVersion::Valid)
    }

    pub fn any_target_version() -> impl Strategy<Value = TargetVersion> {
        arb::version_id().prop_map(TargetVersion::Any)
    }

    pub fn target_version() -> impl Strategy<Value = TargetVersion> {
        prop_oneof![
            4 => valid_target_version(),
            1 => any_target_version(),
        ]
    }

    pub fn versioned_file_target_file() -> impl Strategy<Value = TargetFile> {
        Just(TargetFile::VersionedFile)
    }

    pub fn any_target_file() -> impl Strategy<Value = TargetFile> {
        arb::fs_path_segment().prop_map(TargetFile::Any)
    }

    pub fn target_file() -> impl Strategy<Value = TargetFile> {
        prop_oneof![versioned_file_target_file(), any_target_file()]
    }

    pub fn operation() -> impl Strategy<Value = RepositoryOperation> {
        prop_oneof![
            14 => change_versioned_file(),
            14 => commit(),
            7 => amend(),
            9 => discard(),
            4 => check_out_branch(),
            1 => check_out_version(),
            4 => update_description(),
            4 => restore_version(),
            3 => soft_reset(),
            3 => rebase(),
            5 => create_branch(),
            3 => rename_branch(),
            2 => remove_branch(),
            12 => tidy_up(),
        ]
    }

    pub fn operations() -> impl Strategy<Value = Vec<RepositoryOperation>> {
        vec(operation(), 1..=100)
    }
}
