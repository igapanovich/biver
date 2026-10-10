use crate::helpers::byte_chunk::ByteChunk;
use crate::helpers::repository_action::{FileOperation, RepositoryAction};
use crate::helpers::test_env::TestEnv;
use crate::helpers::version_path::VersionPath;
use biver_core::data::{BranchName, VersionId};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::string::string_regex;

pub fn env() -> impl Strategy<Value = TestEnv> {
    init_then_other_repository_actions().prop_map(TestEnv::from_actions)
}

pub fn bytes() -> impl Strategy<Value = ByteChunk> {
    prop_oneof![
        10 => vec(any::<u8>(), 2..2048).prop_map(ByteChunk::from),
        1 => vec(any::<u8>(), 1).prop_map(ByteChunk::from),
        1 => Just(ByteChunk::from(Vec::new())),
    ]
}

fn repository_action() -> impl Strategy<Value = RepositoryAction> {
    prop_oneof![
        1 => Just(RepositoryAction::Discard),
        4 => Just(RepositoryAction::Commit),
        9 => file_operation().prop_map(RepositoryAction::ModifyVersionedFile),
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
            range_length,
            bytes,
        }
    })
}

fn file_operation_overwrite() -> impl Strategy<Value = FileOperation> {
    bytes().prop_map(FileOperation::Overwrite)
}

fn init_then_other_repository_actions() -> impl Strategy<Value = Vec<RepositoryAction>> {
    (bytes(), vec(repository_action(), 0..100)).prop_map(|(init_bytes, post_init_actions)| {
        let mut result = Vec::new();
        result.push(RepositoryAction::ModifyVersionedFile(
            FileOperation::Overwrite(init_bytes),
        ));
        result.push(RepositoryAction::Init);
        result.extend(post_init_actions);
        result
    })
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

pub fn branch_name_option() -> impl Strategy<Value = Option<BranchName>> {
    prop_oneof![Just(None), branch_name().prop_map(Some)]
}

pub fn version_id() -> impl Strategy<Value = VersionId> {
    any::<u128>().prop_map(VersionId::from_u128)
}
