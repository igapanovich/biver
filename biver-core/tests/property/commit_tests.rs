use crate::helpers::arb;
use biver_core::operations;
use proptest::prelude::*;
use std::fs;

proptest! {
    #[test]
    fn commit_succeeds_on_initialized_repo(env in arb::test_env()) {
        let outcome = operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        prop_assert!(outcome.is_ok() || outcome.is_nothing_to_commit())
    }

    #[test]
    fn commit_after_discard_is_noop(env in arb::test_env()) {
        operations::discard(env.config(), env.paths(), &env.read_repository())?;

        let outcome = operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        prop_assert!(outcome.is_nothing_to_commit())
    }

    #[test]
    fn commit_does_not_modify_versioned_file(env in arb::test_env()) {
        let bytes_before_commit = fs::read(&env.paths().versioned_file)?;

        operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        let bytes_after_commit = fs::read(&env.paths().versioned_file)?;

        prop_assert_eq!(bytes_before_commit, bytes_after_commit);
    }

    #[test]
    fn no_uncommitted_changes_after_commit(env in arb::test_env()) {
        operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        let has_uncommitted_changes = operations::has_uncommitted_changes(env.paths(), &env.read_repository())?;

        prop_assert!(!has_uncommitted_changes);
    }

    #[test]
    fn successful_commit_increases_version_count_by_one(env in arb::test_env(), file_operations in arb::few_file_operations()) {
        let count_before = env.read_repository().versions.len();

        env.run_versioned_file_operations(file_operations);

        let outcome = operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        prop_assume!(outcome.is_ok());

        let count_after = env.read_repository().versions.len();

        prop_assert_eq!(count_after, count_before + 1);
    }

    #[test]
    fn commit_does_not_modify_existing_versions(env in arb::test_env(), file_operations in arb::zero_or_few_file_operations()) {
        let versions_with_content_before = env.read_versions_with_content();

        env.run_versioned_file_operations(file_operations);

        operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        let versions_with_content_after = env.read_versions_with_content();

        prop_assert_eq!(
            &versions_with_content_before,
            &versions_with_content_after.intersection(&versions_with_content_before).cloned().collect()
        );
    }
}
