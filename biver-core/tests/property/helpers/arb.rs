use crate::helpers::repository_action::{FileOperation, PositionInFile, RepositoryAction};
use crate::helpers::test_env::TestEnv;
use crate::helpers::version_path::{VersionPath, VersionPathNode, VersionPathStart};
use proptest::collection::vec;
use proptest::prelude::*;

pub fn bytes() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        10 => vec(any::<u8>(), 2..2048),
        1 => vec(any::<u8>(), 1),
        1 => Just(vec![]),
    ]
}

pub fn non_empty_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        11 => vec(any::<u8>(), 2..2048),
        1 => vec(any::<u8>(), 1),
    ]
}

fn repository_action() -> impl Strategy<Value = RepositoryAction> {
    prop_oneof![
        9 => file_operation().prop_map(RepositoryAction::ModifyVersionedFile),
        3 => Just(RepositoryAction::Commit),
        1 => Just(RepositoryAction::Discard),
    ]
}

pub fn file_operation() -> impl Strategy<Value = FileOperation> {
    prop_oneof![
        12 => file_operation_insert(),
        12 => file_operation_remove_range(),
        1 => file_operation_overwrite(),
    ]
}

pub fn zero_or_few_file_operations() -> impl Strategy<Value = Vec<FileOperation>> {
    vec(file_operation(), 0..5)
}

fn file_operation_insert() -> impl Strategy<Value = FileOperation> {
    (file_position(), non_empty_bytes())
        .prop_map(|(position, bytes)| FileOperation::Insert { position, bytes })
}

fn file_operation_remove_range() -> impl Strategy<Value = FileOperation> {
    (file_position(), 1..1024_usize)
        .prop_map(|(start, length)| FileOperation::RemoveRange { start, length })
}

fn file_operation_overwrite() -> impl Strategy<Value = FileOperation> {
    bytes().prop_map(FileOperation::Overwrite)
}

fn file_position() -> impl Strategy<Value = PositionInFile> {
    prop_oneof![
        4 => any::<usize>().prop_map(PositionInFile::FromStart),
        4 => any::<usize>().prop_map(PositionInFile::FromEnd),
        1 => Just(PositionInFile::FromStart(0)),
        1 => Just(PositionInFile::FromEnd(0)),
    ]
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

pub fn env() -> impl Strategy<Value = TestEnv> {
    init_then_other_repository_actions().prop_map(TestEnv::from_actions)
}

fn version_path_start() -> impl Strategy<Value = VersionPathStart> {
    prop_oneof![
        Just(VersionPathStart::Root),
        Just(VersionPathStart::Head),
        any::<usize>().prop_map(VersionPathStart::TipOfBranch)
    ]
}

fn version_path_node() -> impl Strategy<Value = VersionPathNode> {
    let parent_of = Just(VersionPathNode::Parent);

    let child_of = any::<usize>().prop_map(|child_num| VersionPathNode::Child(child_num));

    prop_oneof![parent_of, child_of]
}

pub fn version_path() -> impl Strategy<Value = VersionPath> {
    (version_path_start(), vec(version_path_node(), 0..100))
        .prop_map(|(start, nodes)| VersionPath { start, nodes })
}
