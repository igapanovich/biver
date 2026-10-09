use crate::helpers::checkout_target::CheckoutTarget;
use crate::helpers::repository_action::{FileOperation, RepositoryAction};
use crate::helpers::test_env::TestEnv;
use crate::helpers::version_path::{VersionPath, VersionPathNode, VersionPathStart};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;

pub fn env() -> impl Strategy<Value = TestEnv> {
    init_then_other_repository_actions().prop_map(TestEnv::from_actions)
}

pub fn checkout_target() -> impl Strategy<Value = CheckoutTarget> {
    prop_oneof![
        3 => any::<Index>().no_shrink().prop_map(CheckoutTarget::Branch),
        1 => version_path().prop_map(CheckoutTarget::Version),
    ]
}

fn bytes() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        10 => vec(any::<u8>(), 2..2048),
        1 => vec(any::<u8>(), 1),
        1 => Just(vec![]),
    ]
}

fn repository_action() -> impl Strategy<Value = RepositoryAction> {
    prop_oneof![
        1 => Just(RepositoryAction::Discard),
        4 => Just(RepositoryAction::Commit),
        1 => checkout_target().prop_map(RepositoryAction::CheckOut),
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

fn version_path_start() -> impl Strategy<Value = VersionPathStart> {
    let tip_of_branch = any::<Index>()
        .no_shrink()
        .prop_map(VersionPathStart::TipOfBranch);

    prop_oneof![
        Just(VersionPathStart::Root),
        Just(VersionPathStart::Head),
        tip_of_branch,
    ]
}

fn version_path_node() -> impl Strategy<Value = VersionPathNode> {
    let parent = Just(VersionPathNode::Parent);

    let child = any::<Index>()
        .no_shrink()
        .prop_map(|child_num| VersionPathNode::Child(child_num));

    prop_oneof![parent, child]
}

fn version_path() -> impl Strategy<Value = VersionPath> {
    (version_path_start(), vec(version_path_node(), 0..100))
        .prop_map(|(start, nodes)| VersionPath { start, nodes })
}
