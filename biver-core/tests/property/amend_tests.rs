use crate::helpers::arb::TestEnvOption::{
    AtLeastTwoBranches, HeadOnBranch, HeadOnVersion, NoUncommittedChanges, SingleBranch,
    UncommittedChanges,
};
use crate::helpers::diff::Diff;
use crate::helpers::snapshot::Snapshot;
use crate::helpers::{arb, assert_version_content_integrity};
use biver_core::repository::{AmendResult, CleanupResult};
use itertools::Itertools;
use proptest::prelude::*;

proptest! {
    #[test]
    fn amend_ok_single_branch_properties(env in arb::env_with_opts(&[UncommittedChanges, HeadOnBranch, SingleBranch]), description in any::<Option<String>>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let result = session.amend(description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, AmendResult::Ok(CleanupResult::Ok), "amend succeeds if there are uncommitted changes and head is on branch and there is only one branch");

        prop_assert!(!after.has_uncommitted_changes, "successful amend does not leave uncommitted changes");
        prop_assert_eq!(&before.head, &after.head, "successful amend does not modify head");
        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "successful amend does not modify versioned file");

        let diff = Snapshot::diff(&before, &after);

        prop_assert_eq!(diff.inserted_versions.len(), 1, "successful amend adds one version");
        prop_assert!(diff.removed_versions.len() <= 1, "successful amend removes at most one version");

        prop_assert!(diff.removed_branches.is_empty(), "successful amend does not remove branches");
        prop_assert_eq!(
            diff.upserted_branches.keys().collect_vec(),
            vec![before.head.branch_ref().unwrap()],
            "successful amend modifies head branch and only it"
        );

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn amend_ok_not_root_properties(env in arb::env_with_opts(&[UncommittedChanges, HeadOnBranch, AtLeastTwoBranches]), description in any::<Option<String>>()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().head_version_id() != session.tree().root_version().id);

        let before = env.snapshot(&session);

        let result = session.amend(description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, AmendResult::Ok(CleanupResult::Ok), "amend succeeds if there are uncommitted changes and head is on branch and not amending root");

        prop_assert!(!after.has_uncommitted_changes, "successful amend does not leave uncommitted changes");
        prop_assert_eq!(&before.head, &after.head, "successful amend does not modify head");
        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "successful amend does not modify versioned file");

        let diff = Snapshot::diff(&before, &after);

        prop_assert_eq!(diff.inserted_versions.len(), 1, "successful amend adds one version");
        prop_assert!(diff.removed_versions.len() <= 1, "successful amend removes at most one version");

        prop_assert!(diff.removed_branches.is_empty(), "successful amend does not remove branches");
        prop_assert_eq!(
            diff.upserted_branches.keys().collect_vec(),
            vec![before.head.branch_ref().unwrap()],
            "successful amend modifies head branch and only it"
        );

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn amend_head_must_be_branch_properties(env in arb::env_with_opts(&[UncommittedChanges, HeadOnVersion]), description in any::<Option<String>>()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().head_version_id() != session.tree().root_version().id);

        let before = env.snapshot(&session);

        let result = session.amend(description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, AmendResult::HeadMustBeOnBranch, "amend fails if head is not pointing at a branch");

        prop_assert_eq!(before, after, "failed amend does not modify anything");

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn amend_no_uncommitted_changes_properties(env in arb::env_with_opts(&[NoUncommittedChanges, HeadOnBranch]), description in any::<Option<String>>()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().head_version_id() != session.tree().root_version().id);

        let before = env.snapshot(&session);

        let result = session.amend(description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, AmendResult::NoUncommittedChanges, "amend fails if there are no uncommitted changes");

        prop_assert_eq!(before, after, "failed amend does not modify anything");

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn amend_cannot_amend_root_when_other_branches_exist_properties(env in arb::env_with_opts(&[NoUncommittedChanges]), description in any::<Option<String>>(), new_branch_name in arb::branch_name(), insert in arb::file_operation_non_empty_insert()) {
        let mut session = env.start_session();

        prop_assume!(!session.tree().branch_names().contains(&new_branch_name));

        session.create_branch(new_branch_name.clone(), session.tree().root_version().id)?;
        session.check_out_branch(new_branch_name)?;
        insert.execute(env.versioned_file_path());

        let before = env.snapshot(&session);

        let result = session.amend(description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, AmendResult::CannotAmendRootWhenOtherBranchesExist, "amend fails if trying to amend root and other branches exist");

        prop_assert_eq!(before, after, "failed amend does not modify anything");

        assert_version_content_integrity(session)?;
    }
}
