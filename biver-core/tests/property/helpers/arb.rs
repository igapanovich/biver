use crate::helpers::byte_chunk::ByteChunk;
use crate::helpers::file_operation::FileOperation;
use crate::helpers::repository_blueprint::{
    BranchBlueprint, HeadBlueprint, InitialBranchBlueprint, RepositoryBlueprint, VersionBlueprint,
};
use crate::helpers::test_env::TestEnv;
use crate::helpers::version_path::VersionPath;
use biver_core::data::{BranchName, VersionId};
use proptest::collection::vec;
use proptest::option;
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::string::string_regex;
use std::path::PathBuf;

pub fn env() -> impl Strategy<Value = TestEnv> {
    repository_blueprint(&[]).prop_map(TestEnv::from_blueprint)
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TestEnvOption {
    UncommittedChanges,
    NoUncommittedChanges,
    HeadOnBranch,
    HeadOnVersion,
    AtLeastTwoBranches,
    SingleBranch,
}

pub fn env_with_opts(options: &[TestEnvOption]) -> impl Strategy<Value = TestEnv> {
    repository_blueprint(options).prop_map(TestEnv::from_blueprint)
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

    let other_branches = if options.contains(&TestEnvOption::AtLeastTwoBranches) {
        vec(branch_blueprint(), 1..=4).boxed()
    } else if options.contains(&TestEnvOption::SingleBranch) {
        Just(Vec::new()).boxed()
    } else {
        vec(branch_blueprint(), 0..=4).boxed()
    };

    (
        initial_branch_blueprint(),
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

fn initial_branch_blueprint() -> impl Strategy<Value = InitialBranchBlueprint> {
    (option::of(branch_name()), vec(version_blueprint(), 1..=8)).prop_map(|(name, mut versions)| {
        InitialBranchBlueprint {
            name,
            first_version: versions.remove(0),
            following_versions: versions,
        }
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
        .prop_map(|s| PathBuf::from(s))
}

pub fn fs_path() -> impl Strategy<Value = PathBuf> {
    vec(fs_path_segment(), 1..=8).prop_map(|mut segments| {
        let mut path = segments.pop().unwrap();

        while segments.len() > 0 {
            path = path.join(segments.pop().unwrap());
        }

        path
    })
}
