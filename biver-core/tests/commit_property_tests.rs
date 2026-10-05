use crate::property_based::arb_repository_action::arb_repository_actions;
use crate::property_based::arb_versioned_file_content::arb_versioned_file_content;
use crate::property_based::test_env;
use biver_core::operations;
use proptest::prelude::*;
use std::fs;

mod common;
mod property_based;

proptest! {
    #[test]
    fn commit_succeeds_on_initialized_repo(actions in arb_repository_actions(), new_bytes in arb_versioned_file_content()) {
        let mut env = test_env::create_with_actions(actions);

        fs::write(&env.paths.versioned_file, new_bytes).unwrap();

        let outcome = operations::commit(&env.config, &env.paths, &mut env.repo, None).unwrap();

        assert!(outcome.is_ok() || outcome.is_nothing_to_commit())
    }

    #[test]
    fn commit_after_discard_is_noop(actions in arb_repository_actions(), new_bytes in arb_versioned_file_content()) {
        let mut env = test_env::create_with_actions(actions);

        fs::write(&env.paths.versioned_file, new_bytes).unwrap();

        operations::discard(&env.config, &env.paths, &env.repo).unwrap();

        let outcome = operations::commit(&env.config, &env.paths, &mut env.repo, None).unwrap();

        assert!(outcome.is_nothing_to_commit())
    }

    #[test]
    fn commit_does_not_modify_versioned_file(actions in arb_repository_actions(), new_bytes in arb_versioned_file_content()) {
        let mut env = test_env::create_with_actions(actions);

        fs::write(&env.paths.versioned_file, &new_bytes).unwrap();

        operations::commit(&env.config, &env.paths, &mut env.repo, None).unwrap();

        let bytes_after_commit = fs::read(&env.paths.versioned_file).unwrap();

        assert_eq!(new_bytes, bytes_after_commit);
    }
}
