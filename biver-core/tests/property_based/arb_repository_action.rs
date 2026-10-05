#![allow(dead_code)]

use crate::property_based::arb_versioned_file_content::arb_versioned_file_content;
use crate::property_based::repository_action::RepositoryAction;
use proptest::collection::vec;
use proptest::prelude::*;

prop_compose! {
    pub fn arb_repository_actions()(init in arb_init(), items in vec(arb_initialized_repository_action(), 0..100)) -> Vec<RepositoryAction> {
        let mut result = Vec::new();
        result.push(init);
        result.extend(items);
        result
    }
}

pub fn arb_initialized_repository_action() -> impl Strategy<Value = RepositoryAction> {
    prop_oneof![
        10 => arb_modify_versioned_file(),
        10 => arb_commit()
    ]
}

prop_compose! {
    fn arb_commit_with_new_bytes()(bytes in arb_versioned_file_content()) -> RepositoryAction {
        RepositoryAction::Commit(Some(bytes))
    }
}

pub fn arb_commit() -> impl Strategy<Value = RepositoryAction> {
    prop_oneof![
        3 => arb_commit_with_new_bytes(),
        1 => Just(RepositoryAction::Commit(None))
    ]
}

prop_compose! {
    pub fn arb_init()(bytes in arb_versioned_file_content()) -> RepositoryAction {
        RepositoryAction::Init(bytes)
    }
}

prop_compose! {
    pub fn arb_modify_versioned_file()(bytes in arb_versioned_file_content()) -> RepositoryAction {
        RepositoryAction::ModifyVersionedFile(bytes)
    }
}
