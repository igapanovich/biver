use crate::helpers::arb::TestEnvOption::{HeadOnBranch, HeadOnVersion};
use crate::helpers::diff::Diff;
use crate::helpers::extensions::{IdsExt, IndexExt, IteratorExt};
use crate::helpers::snapshot::Snapshot;
use crate::helpers::{arb, assert_repository_integrity, set};
use biver_core::repository::{CleanupResult, SoftResetResult};
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #[test]
    fn soft_reset_ok_properties(env in arb::env_with_opts(&[HeadOnBranch]), version_index in any::<Index>()) {
        let mut session = env.start_session();

        let head_branch = session.tree().head_branch().cloned().unwrap();
        let head_ancestors = session.tree().head_and_ancestors().skip(1).collect_vec();

        prop_assume!(!head_ancestors.is_empty());

        let versions_exclusive_to_head = session.tree().branch_tip_and_ancestors_exclusive(&head_branch).unwrap().cloned().collect_set();

        let target_version_id = version_index.get_copied(&head_ancestors).id;

        let before = env.snapshot(&session);

        let result = session.soft_reset(target_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, SoftResetResult::Ok(CleanupResult::Ok), "soft_reset succeeds if head is on a branch and target version is among head version ancestors");

        prop_assert_eq!(&before.head, &after.head, "successful soft_reset does not modify head");
        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "successful soft_reset does not modify versioned file");

        let diff = Snapshot::diff(&before, &after);

        prop_assert!(diff.inserted_versions.is_empty(), "successful soft_reset does not add versions");
        prop_assert_eq!(diff.removed_versions.iter().ids().difference(&versions_exclusive_to_head.iter().ids()).count(), 0, "successful soft_reset only removes versions exclusively owned by head branch");
        prop_assert_eq!(diff.upserted_branches.keys().collect_set(), set![&head_branch], "successful soft_reset only modifies head branch");
        prop_assert!(diff.removed_branches.is_empty(), "successful soft_reset does not remove branches");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn soft_reset_head_branch_tip_is_already_there_properties(env in arb::env_with_opts(&[HeadOnBranch])) {
        let mut session = env.start_session();

        let target_version_id = session.tree().head_version_id();

        let before = env.snapshot(&session);

        let result = session.soft_reset(target_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, SoftResetResult::HeadBranchTipIsAlreadyThere, "soft_reset is noop if target version is the head version");

        prop_assert_eq!(before, after, "noop soft_reset does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn soft_reset_head_must_be_on_branch_properties(env in arb::env_with_opts(&[HeadOnVersion]), version_index in any::<Index>()) {
        let mut session = env.start_session();

        let head_ancestors = session.tree().head_and_ancestors().skip(1).collect_vec();

        prop_assume!(!head_ancestors.is_empty());

        let target_version_id = version_index.get_copied(&head_ancestors).id;

        let before = env.snapshot(&session);

        let result = session.soft_reset(target_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, SoftResetResult::HeadMustBeOnBranch, "soft_reset fails if head is not pointing at a branch");

        prop_assert_eq!(before, after, "failed soft_reset does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn soft_reset_target_not_head_ancestor_properties(env in arb::env_with_opts(&[HeadOnBranch]), version_index in any::<Index>()) {
        let mut session = env.start_session();

        let head_ancestor_ids = session.tree().head_and_ancestors().into_ids();
        let mut versions_except_head_ancestors = session.tree().versions().collect_vec();
        versions_except_head_ancestors.retain(|v| !head_ancestor_ids.contains(&v.id));

        prop_assume!(!versions_except_head_ancestors.is_empty());

        let target_version_id = version_index.get_copied(&versions_except_head_ancestors).id;

        let before = env.snapshot(&session);

        let result = session.soft_reset(target_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, SoftResetResult::InvalidStopAtVersionId, "soft_reset fails if target version is not a head ancestor");

        prop_assert_eq!(before, after, "failed soft_reset does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn soft_reset_target_not_found_properties(env in arb::env_with_opts(&[HeadOnBranch]), target_version_id in arb::version_id()) {
        let mut session = env.start_session();

        prop_assume!(!session.tree().versions().into_ids().contains(&target_version_id));

        let before = env.snapshot(&session);

        let result = session.soft_reset(target_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, SoftResetResult::InvalidStopAtVersionId, "soft_reset fails if target version was not found");

        prop_assert_eq!(before, after, "failed soft_reset does not modify anything");

        assert_repository_integrity(session)?;
    }
}
