use crate::helpers::arb::TestEnvOption::AtLeastTwoBranches;
use crate::helpers::diff::Diff;
use crate::helpers::extensions::IndexExt;
use crate::helpers::test_env_snapshot::TestEnvSnapshot;
use crate::helpers::{arb, assert_repository_integrity};
use biver_core::repository::RenameBranchResult;
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #[test]
    fn rename_branch_ok_properties(test_env in arb::test_env(), old_branch_index in any::<Index>(), new_branch_name in arb::branch_name()) {
        let mut session = test_env.start_session();

        prop_assume!(!session.tree().branch_names().contains(&new_branch_name));
        let old_branch_name = old_branch_index.get_copied(&session.tree().branch_names().collect_vec()).clone();

        let before = test_env.snapshot(&session);

        let result = session.rename_branch(&old_branch_name, new_branch_name.clone())?;

        let after = test_env.snapshot(&session);

        prop_assert_eq!(result, RenameBranchResult::Ok, "rename_branch succeeds if old name is valid and new name doesn't exist");
        prop_assert_eq!(&before.versions, &after.versions, "rename_branch does not modify versions");
        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "rename_branch does not modify versioned file");

        if before.head.branch_ref() == Some(&old_branch_name) {
            prop_assert_eq!(after.head.branch_ref(), Some(&new_branch_name), "rename_branch updates head if it pointed at the renamed branch");
        } else {
            prop_assert_eq!(&before.head, &after.head, "rename_branch does not modify head if it wasn't pointing at the renamed branch");
        }

        let diff = TestEnvSnapshot::diff(&before, &after);

        prop_assert_eq!(diff.upserted_branches.len(), 1, "rename_branch creates one branch");
        prop_assert_eq!(diff.upserted_branches.keys().next(), Some(&new_branch_name), "rename_branch creates a branch with the provided name");
        prop_assert_eq!(diff.removed_branches.len(), 1, "rename_branch removes one branch");
        prop_assert!(diff.removed_branches.contains(&old_branch_name), "rename_branch removes the branch with the old name");

        drop(session);
        assert_repository_integrity(&test_env)?;
    }

    #[test]
    fn rename_branch_new_name_already_exists_properties(test_env in arb::test_env_with_opts(&[AtLeastTwoBranches]), old_branch_index in any::<Index>(), new_branch_index in any::<Index>()) {
        let mut session = test_env.start_session();

        let mut branch_names = session.tree().branch_names().collect_vec();
        let old_branch_index = old_branch_index.index(branch_names.len());
        let old_branch_name = branch_names.remove(old_branch_index).clone();
        let new_branch_name = new_branch_index.get_copied(&branch_names).clone();

        let before = test_env.snapshot(&session);

        let result = session.rename_branch(&old_branch_name, new_branch_name)?;

        let after = test_env.snapshot(&session);

        prop_assert_eq!(result, RenameBranchResult::NewNameAlreadyExists, "rename_branch fails if the new branch name already exists");

        prop_assert_eq!(before, after, "failed rename_branch does not modify anything");

        drop(session);
        assert_repository_integrity(&test_env)?;
    }

    #[test]
    fn rename_branch_old_name_not_found_properties(test_env in arb::test_env(), old_branch_name in arb::branch_name(), new_branch_name in arb::branch_name()) {
        let mut session = test_env.start_session();

        prop_assume!(!session.tree().branch_names().contains(&old_branch_name));
        prop_assume!(!session.tree().branch_names().contains(&new_branch_name));

        let before = test_env.snapshot(&session);

        let result = session.rename_branch(&old_branch_name, new_branch_name)?;

        let after = test_env.snapshot(&session);

        prop_assert_eq!(result, RenameBranchResult::OldNameNotFound, "rename_branch fails if the old branch name was not found");

        prop_assert_eq!(before, after, "failed rename_branch does not modify anything");

        drop(session);
        assert_repository_integrity(&test_env)?;
    }
}
