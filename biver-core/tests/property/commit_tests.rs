use crate::helpers::arb::TestEnvOption::{
    HeadOnBranch, HeadOnVersion, NoUncommittedChanges, UncommittedChanges,
};
use crate::helpers::diff::Diff;
use crate::helpers::snapshot::Snapshot;
use crate::helpers::{arb, assert_version_content_integrity};
use biver_core::repository::CommitResult;
use itertools::Itertools;
use proptest::prelude::*;

proptest! {
    #[test]
    fn commit_ok_properties(env in arb::env_with_opts(&[HeadOnBranch, UncommittedChanges]), description in any::<String>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let result = session.commit(description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CommitResult::Ok, "commit succeeds if head is on a branch and there are uncommitted changes");

        prop_assert!(!after.has_uncommitted_changes, "successful commit does not leave uncommitted changes");
        prop_assert_eq!(&before.head, &after.head, "successful commit does not modify head");
        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "successful commit does not modify versioned file");

        let diff = Snapshot::diff(&before, &after);

        prop_assert_eq!(diff.inserted_versions.len(), 1, "successful commit adds one versions");
        prop_assert!(diff.removed_versions.is_empty(), "successful commit does not remove versions");

        prop_assert!(diff.removed_branches.is_empty(), "successful commit does not remove branches");
        prop_assert_eq!(
            diff.upserted_branches.keys().collect_vec(),
            vec![before.head.branch_ref().unwrap()],
            "successful commit modifies head branch and only it"
        );

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn commit_head_must_be_on_branch_properties(env in arb::env_with_opts(&[HeadOnVersion, UncommittedChanges]), description in any::<String>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let result = session.commit(description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CommitResult::HeadMustBeOnBranch, "commit fails if head is not pointing at a branch");

        prop_assert_eq!(before, after, "failed commit does not modify anything");

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn commit_no_uncommitted_changes_properties(env in arb::env_with_opts(&[HeadOnBranch, NoUncommittedChanges]), description in any::<String>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let result = session.commit(description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CommitResult::NoUncommittedChanges, "commit is noop if there are no uncommitted changes");

        prop_assert_eq!(before, after, "noop commit does not modify anything");

        assert_version_content_integrity(session)?;
    }
}
