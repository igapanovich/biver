use crate::helpers::arb::TestEnvOption::AtLeastTwoBranches;
use crate::helpers::diff::Diff;
use crate::helpers::extensions::{IdsExt, IndexExt, IteratorExt};
use crate::helpers::snapshot::Snapshot;
use crate::helpers::{arb, assert_version_content_integrity, set};
use biver_core::repository::{CleanupResult, RemoveBranchResult};
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #[test]
    fn remove_branch_ok_properties(env in arb::env_with_opts(&[AtLeastTwoBranches]), branch_index in any::<Index>()) {
        let mut session = env.start_session();

        let head_branch = session.tree().head_branch();
        let branches_except_head = session.tree().branch_names().except_some(&head_branch).collect_vec();

        let branch_to_remove = branch_index.get_copied(&branches_except_head).clone();

        let head_version_id = session.tree().head_version_id();
        let versions_owned_only_by_removed_branch = session.tree().branch_tip_and_ancestors_exclusive(&branch_to_remove).unwrap().cloned().collect_vec();

        prop_assume!(versions_owned_only_by_removed_branch.iter().all(|v| v.id != head_version_id));

        let before = env.snapshot(&session);

        let result = session.remove_branch(&branch_to_remove)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RemoveBranchResult::Ok(CleanupResult::Ok), "remove_branch succeeds if there are at least two branches and head version is unaffected");

        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "successful remove_branch does not modify versioned file");

        let diff = Snapshot::diff(&before, &after);

        prop_assert_eq!(diff.removed_versions.iter().ids(), versions_owned_only_by_removed_branch.iter().ids(), "successful remove_branch only removes versions exclusively owned by the removed branch");
        prop_assert!(diff.inserted_versions.is_empty(), "successful remove_branch does not create new versions");

        prop_assert_eq!(diff.removed_branches, set![branch_to_remove], "successful remove_branch only removes the target branch");
        prop_assert!(diff.upserted_branches.is_empty(), "successful remove_branch does not create branches");

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn remove_branch_cannot_remove_head_branch_properties(env in arb::env_with_opts(&[AtLeastTwoBranches])) {
        let mut session = env.start_session();

        prop_assume!(session.tree().head_branch().is_some());
        let head_branch = session.tree().head_branch().unwrap().clone();

        let before = env.snapshot(&session);

        let result = session.remove_branch(&head_branch)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RemoveBranchResult::CannotRemoveHeadBranch, "remove_branch fails if target branch is head branch");

        prop_assert_eq!(before, after, "failed remove_branch does not modify anything");

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn remove_branch_cannot_remove_head_version_properties(env in arb::env_with_opts(&[AtLeastTwoBranches]), branch_index in any::<Index>(), check_out_index in any::<Index>()) {
        let mut session = env.start_session();

        let branch_name = branch_index.get_copied(&session.tree().branch_names().collect_vec()).clone();

        let exclusive_versions = session.tree().branch_tip_and_ancestors_exclusive(&branch_name).unwrap().map(|v| v.id).collect_vec();

        prop_assume!(!exclusive_versions.is_empty());

        let check_out_id = check_out_index.get_copied(&exclusive_versions);

        session.check_out_version(check_out_id)?;

        let before = env.snapshot(&session);

        let result = session.remove_branch(&branch_name)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RemoveBranchResult::CannotRemoveHeadVersion, "remove_branch fails if it would result in removing head version");

        prop_assert_eq!(before, after, "failed remove_branch does not modify anything");

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn remove_branch_branch_not_found_properties(env in arb::env_with_opts(&[AtLeastTwoBranches]), branch_name in arb::branch_name()) {
        let mut session = env.start_session();

        prop_assume!(!session.tree().branch_names().contains(&branch_name));

        let before = env.snapshot(&session);

        let result = session.remove_branch(&branch_name)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RemoveBranchResult::BranchNotFound, "remove_branch fails if target branch was not found");

        prop_assert_eq!(before, after, "failed remove_branch does not modify anything");

        assert_version_content_integrity(session)?;
    }
}
