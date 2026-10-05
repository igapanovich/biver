use crate::helpers::arb;
use biver_core::operations;
use proptest::prelude::*;
use std::fs;

proptest! {
    #[test]
    fn commit_succeeds_on_initialized_repo(env in arb::test_env()) {
        let outcome = operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        assert!(outcome.is_ok() || outcome.is_nothing_to_commit())
    }

    #[test]
    fn commit_after_discard_is_noop(env in arb::test_env()) {
        operations::discard(env.config(), env.paths(), &env.read_repository())?;

        let outcome = operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        assert!(outcome.is_nothing_to_commit())
    }

    #[test]
    fn commit_does_not_modify_versioned_file(env in arb::test_env()) {
        let bytes_before_commit = fs::read(&env.paths().versioned_file)?;

        operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        let bytes_after_commit = fs::read(&env.paths().versioned_file)?;

        assert_eq!(bytes_before_commit, bytes_after_commit);
    }

    #[test]
    fn no_uncommitted_changes_after_commit(env in arb::test_env()) {
        operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        let has_uncommitted_changes = operations::has_uncommitted_changes(env.paths(), &env.read_repository())?;

        assert!(!has_uncommitted_changes);
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
    fn commit_does_not_change_existing_versions(env in arb::test_env(), file_operations in arb::zero_or_few_file_operations()) {
        let existing_versions_before = env.read_repository().versions;
        let existing_content_blobs_before = existing_versions_before.iter()
            .map(|v| env.paths().file_path(&v.content_blob_file_name))
            .map(|path| fs::read(&path).map(|content| (path, content)))
            .collect::<Result<Vec<_>, _>>()?;

        env.run_versioned_file_operations(file_operations);

        operations::commit(env.config(), env.paths(), &mut env.read_repository(), None)?;

        let mut existing_versions_after = env.read_repository().versions;
        existing_versions_after.retain(|v| existing_versions_before.iter().any(|other| v.id == other.id));

        let existing_content_blobs_after = existing_versions_after.iter()
            .map(|v| env.paths().file_path(&v.content_blob_file_name))
            .map(|path| fs::read(&path).map(|content| (path, content)))
            .collect::<Result<Vec<_>, _>>()?;

        prop_assert_eq!(existing_versions_before, existing_versions_after);
        prop_assert_eq!(existing_content_blobs_before, existing_content_blobs_after);
    }
}
