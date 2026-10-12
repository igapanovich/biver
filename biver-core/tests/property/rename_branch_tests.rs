use crate::helpers::arb;
use crate::helpers::diff::Diff;
use crate::helpers::extensions::IndexExt;
use crate::helpers::snapshot::Snapshot;
use biver_core::repository::RenameBranchResult;
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #[test]
    fn rename_branch_ok_properties(env in arb::env(), old_branch_index in any::<Index>(), new_branch_name in arb::branch_name()) {
        let mut session = env.start_session();

        prop_assume!(!session.tree().branch_names().contains(&new_branch_name));
        let old_branch_name = old_branch_index.get_copied(&session.tree().branch_names().collect_vec()).clone();

        let before = env.snapshot(&session);

        let result = session.rename_branch(old_branch_name.clone(), new_branch_name.clone())?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RenameBranchResult::Ok, "rename_branch succeeds if old name is valid and new name doesn't exist");
        prop_assert_eq!(&before.versions, &after.versions, "rename_branch does not modify versions");
        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "rename_branch does not modify versioned file");

        if before.head.branch() == Some(&old_branch_name) {
            prop_assert_eq!(after.head.branch(), Some(&new_branch_name), "rename_branch updates head if it pointed at the renamed branch");
        } else {
            prop_assert_eq!(&before.head, &after.head, "rename_branch does not modify head if it wasn't pointing at the renamed branch");
        }

        let diff = Snapshot::diff(&before, &after);

        prop_assert_eq!(diff.upserted_branches.len(), 1, "rename_branch creates one branch");
        prop_assert_eq!(diff.upserted_branches.keys().next(), Some(&new_branch_name), "rename_branch creates a branch with the provided name");
        prop_assert_eq!(diff.removed_branches.len(), 1, "rename_branch removes one branch");
        prop_assert!(diff.removed_branches.contains(&old_branch_name), "rename_branch removes the branch with the old name");
    }

    #[test]
    fn rename_branch_new_name_already_exists_properties(env in arb::env(), old_branch_index in any::<Index>(), new_branch_index in any::<Index>()) {
        let mut session = env.start_session();

        let new_branch_name = new_branch_index.get_copied(&session.tree().branch_names().collect_vec()).clone();
        let old_branch_name = old_branch_index.get_copied(&session.tree().branch_names().collect_vec()).clone();

        prop_assume!(new_branch_name != old_branch_name);

        let before = env.snapshot(&session);

        let result = session.rename_branch(old_branch_name, new_branch_name)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RenameBranchResult::NewNameAlreadyExists, "rename_branch fails if the new branch name already exists");

        prop_assert_eq!(before, after, "failed rename_branch does not modify anything");
    }

    #[test]
    fn rename_branch_old_name_not_found_properties(env in arb::env(), old_branch_name in arb::branch_name(), new_branch_name in arb::branch_name()) {
        let mut session = env.start_session();

        prop_assume!(!session.tree().branch_names().contains(&old_branch_name));
        prop_assume!(!session.tree().branch_names().contains(&new_branch_name));

        let before = env.snapshot(&session);

        let result = session.rename_branch(old_branch_name, new_branch_name)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RenameBranchResult::OldBranchNotFound, "rename_branch fails if the old branch name was not found");

        prop_assert_eq!(before, after, "failed rename_branch does not modify anything");
    }
}
