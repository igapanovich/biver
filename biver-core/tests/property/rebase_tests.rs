use crate::helpers::arb::TestEnvOption::{AtLeastTwoVersions, HeadOnBranch, HeadOnVersion};
use crate::helpers::diff::Diff;
use crate::helpers::extensions::IdsExt;
use crate::helpers::snapshot::Snapshot;
use crate::helpers::{arb, assert_repository_integrity};
use biver_core::repository::{CleanupResult, RebaseResult};
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #[test]
    fn rebase_ok_properties(env in arb::env_with_opts(&[HeadOnBranch, AtLeastTwoVersions]), rebased_version_index in any::<Index>(), new_parent_version_index in any::<Index>()) {
        let mut session = env.start_session();

        let all_versions = session.tree().versions().collect_vec();
        let head_branch_versions_except_root = session.tree().head_and_ancestors().take_while(|v| !v.is_root()).collect_vec();

        prop_assume!(!head_branch_versions_except_root.is_empty());

        let rebased_version_index = rebased_version_index.index(head_branch_versions_except_root.len());
        let rebased_version_count = rebased_version_index + 1;
        let head_branch = session.tree().head_branch().unwrap();
        let version_ids_exclusive_to_head_branch = session.tree().branch_tip_and_ancestors_exclusive(head_branch).unwrap().into_ids();

        let rebased_version_id = head_branch_versions_except_root[rebased_version_index].id;
        let new_parent_version_id = new_parent_version_index.get(&all_versions).id;

        let before = env.snapshot(&session);

        let result = session.rebase(rebased_version_id, new_parent_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RebaseResult::Ok(CleanupResult::Ok), "rebase succeeds if head points at a branch and rebased version belongs to that branch and it is not root");

        prop_assert_eq!(&before.head, &after.head, "successful rebase does not modify head");
        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "successful rebase does not modify versioned file");

        let diff = Snapshot::diff(&before, &after);

        prop_assert_eq!(diff.removed_branches.len(), 0, "successful rebase does not remove branches");
        prop_assert_eq!(diff.upserted_branches.len(), 1, "successful rebase updates one branch");

        prop_assert_eq!(diff.inserted_versions.len(), rebased_version_count, "successful rebase copies rebased versions");
        prop_assert_eq!(diff.removed_versions.iter().into_ids().difference(&version_ids_exclusive_to_head_branch).count(), 0, "successful rebase may only remove versions exclusively belonging to head branch");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn rebase_head_must_be_on_branch_properties(env in arb::env_with_opts(&[HeadOnVersion, AtLeastTwoVersions]), rebased_version_index in any::<Index>(), new_parent_version_index in any::<Index>()) {
        let mut session = env.start_session();

        let all_versions = session.tree().versions().collect_vec();
        let head_branch_versions_except_root = session.tree().head_and_ancestors().take_while(|v| !v.is_root()).collect_vec();

        prop_assume!(!head_branch_versions_except_root.is_empty());

        let rebased_version_id = rebased_version_index.get(&head_branch_versions_except_root).id;
        let new_parent_version_id = new_parent_version_index.get(&all_versions).id;

        let before = env.snapshot(&session);

        let result = session.rebase(rebased_version_id, new_parent_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RebaseResult::HeadMustBeOnBranch, "rebase fails if head points at a version");

        prop_assert_eq!(before, after, "failed rebase does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn rebase_invalid_rebased_version_properties(env in arb::env_with_opts(&[HeadOnBranch, AtLeastTwoVersions]), rebased_version_id in arb::version_id(), new_parent_version_index in any::<Index>()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().version(rebased_version_id).is_none());

        let all_versions = session.tree().versions().collect_vec();
        let head_branch_versions_except_root = session.tree().head_and_ancestors().take_while(|v| !v.is_root()).collect_vec();

        prop_assume!(!head_branch_versions_except_root.is_empty());

        let new_parent_version_id = new_parent_version_index.get(&all_versions).id;

        let before = env.snapshot(&session);

        let result = session.rebase(rebased_version_id, new_parent_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RebaseResult::InvalidRebasedVersion, "rebase fails if rebased version is invalid");

        prop_assert_eq!(before, after, "failed rebase does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn rebase_invalid_new_parent_version_properties(env in arb::env_with_opts(&[HeadOnBranch, AtLeastTwoVersions]), rebased_version_index in any::<Index>(), new_parent_version_id in arb::version_id()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().version(new_parent_version_id).is_none());

        let head_branch_versions_except_root = session.tree().head_and_ancestors().take_while(|v| !v.is_root()).collect_vec();

        prop_assume!(!head_branch_versions_except_root.is_empty());

        let rebased_version_id = rebased_version_index.get(&head_branch_versions_except_root).id;

        let before = env.snapshot(&session);

        let result = session.rebase(rebased_version_id, new_parent_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RebaseResult::InvalidNewParentVersion, "rebase fails if new parent version is invalid");

        prop_assert_eq!(before, after, "failed rebase does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn rebase_cannot_rebase_root_properties(env in arb::env_with_opts(&[HeadOnBranch]), new_parent_version_index in any::<Index>()) {
        let mut session = env.start_session();

        let all_versions = session.tree().versions().collect_vec();

        let rebased_version_id = session.tree().root_version().id;
        let new_parent_version_id = new_parent_version_index.get(&all_versions).id;

        let before = env.snapshot(&session);

        let result = session.rebase(rebased_version_id, new_parent_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RebaseResult::CannotRebaseRoot, "rebase fails if rebased version is root");

        prop_assert_eq!(before, after, "failed rebase does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn rebase_rebased_version_does_not_belong_to_head_branch_properties(env in arb::env_with_opts(&[HeadOnBranch, AtLeastTwoVersions]), rebased_version_index in any::<Index>(), new_parent_version_index in any::<Index>()) {
        let mut session = env.start_session();

        let all_versions = session.tree().versions().collect_vec();
        let head_branch_version_ids = session.tree().head_and_ancestors().into_ids();
        let versions_except_head_branch = all_versions.iter().filter(|v| !head_branch_version_ids.contains(&v.id)).collect_vec();

        prop_assume!(!versions_except_head_branch.is_empty());

        let rebased_version_id = rebased_version_index.get(&versions_except_head_branch).id;
        let new_parent_version_id = new_parent_version_index.get(&all_versions).id;

        let before = env.snapshot(&session);

        let result = session.rebase(rebased_version_id, new_parent_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RebaseResult::RebasedVersionDoesNotBelongToHeadBranch, "rebase fails if rebased version doesn't belong to head branch");

        prop_assert_eq!(before, after, "failed rebase does not modify anything");

        assert_repository_integrity(session)?;
    }
}
