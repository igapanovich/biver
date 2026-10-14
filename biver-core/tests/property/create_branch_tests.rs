use crate::helpers::arb;
use crate::helpers::diff::Diff;
use crate::helpers::extensions::IndexExt;
use crate::helpers::snapshot::Snapshot;
use crate::helpers::version_path::ResolveVersionPathExt;
use biver_core::repository::CreateBranchResult;
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #[test]
    fn create_branch_ok_properties(env in arb::env(), valid_branch_name in arb::branch_name(), branch_tip_path in arb::version_path()) {
        let mut session = env.start_session();

        prop_assume!(!session.tree().branch_names().contains(&valid_branch_name));

        let valid_branch_tip_id = session.tree().resolve_version_path(&branch_tip_path).id;

        let before = env.snapshot(&session);

        let result = session.create_branch(valid_branch_name.clone(), valid_branch_tip_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CreateBranchResult::Ok, "create_branch succeeds if branch name and branch tip ID are valid");

        prop_assert_eq!(&before.head, &after.head, "successful create_branch does not modify head");
        prop_assert_eq!(&before.versions, &after.versions, "successful create_branch does not modify versions");
        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "successful create_branch does not modify versioned file");

        let diff = Snapshot::diff(&before, &after);

        prop_assert_eq!(diff.upserted_branches.len(), 1, "successful create_branch creates one branch");
        prop_assert_eq!(diff.upserted_branches.keys().next(), Some(&valid_branch_name), "successful create_branch creates a branch with the provided name");
        prop_assert_eq!(diff.upserted_branches.values().next(), Some(&valid_branch_tip_id), "successful create_branch creates a branch at the provided version ID");
        prop_assert!(diff.removed_branches.is_empty(), "successful create_branch does not remove branches");
    }

    #[test]
    fn create_branch_already_exists_properties(env in arb::env(), branch_name_index in any::<Index>(), branch_tip_path in arb::version_path()) {
        let mut session = env.start_session();

        let invalid_branch_name = branch_name_index.get_copied(&session.tree().branch_names().collect_vec()).clone();
        let valid_branch_tip = session.tree().resolve_version_path(&branch_tip_path);

        let before = env.snapshot(&session);

        let result = session.create_branch(invalid_branch_name, valid_branch_tip.id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CreateBranchResult::AlreadyExists, "create_branch fails if the branch name already exists");

        prop_assert_eq!(before, after, "failed create_branch does not modify anything");
    }

    #[test]
    fn create_branch_invalid_branch_tip_properties(env in arb::env(), valid_branch_name in arb::branch_name(), invalid_branch_tip_id in arb::version_id()) {
        let mut session = env.start_session();

        prop_assume!(!session.tree().branch_names().contains(&valid_branch_name));
        prop_assume!(session.tree().version(invalid_branch_tip_id).is_none());

        let before = env.snapshot(&session);

        let result = session.create_branch(valid_branch_name, invalid_branch_tip_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CreateBranchResult::InvalidBranchTipId, "create_branch fails if branch tip ID is invalid");

        prop_assert_eq!(before, after, "failed create_branch does not modify anything");
    }
}
