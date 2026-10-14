use crate::helpers::extensions::IndexExt;
use crate::helpers::{arb, assert_version_content_integrity};
use biver_core::data::Head;
use biver_core::repository::CheckOutBranchResult;
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #[test]
    fn check_out_branch_ok_properties(env in arb::env(), branch_index in any::<Index>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let branch_names = session.tree().branch_names().collect_vec();
        let valid_branch_name = branch_index.get_copied(&branch_names).clone();

        let result = session.check_out_branch(valid_branch_name.clone())?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CheckOutBranchResult::Ok, "check_out_branch succeeds if target branch is valid");

        if before.has_uncommitted_changes {
            prop_assert_eq!(before.versioned_file_content, after.versioned_file_content, "successful check_out_branch does not modify versioned file if there were uncommitted changes");
        } else {
            prop_assert!(!after.has_uncommitted_changes, "successful check_out_branch restores versioned file to the target version if there were no uncommitted changes");
        }

        prop_assert_eq!(after.head, Head::Branch(valid_branch_name), "successful check_out_branch makes head point at the target branch");
        prop_assert_eq!(&before.branches, &after.branches, "successful check_out_branch does not modify branches");
        prop_assert_eq!(&before.versions, &after.versions, "successful check_out_branch does not modify versions");

        assert_version_content_integrity(session)?;
    }

    #[test]
    fn check_out_branch_branch_not_found_properties(env in arb::env(), branch_name in arb::branch_name()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().branch_names().all(|b| b != &branch_name));

        let before = env.snapshot(&session);

        let result = session.check_out_branch(branch_name)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CheckOutBranchResult::BranchNotFound, "check_out_branch fails if branch is not found");

        prop_assert_eq!(before, after, "failed check_out_branch does not modify anything");

        assert_version_content_integrity(session)?;
    }
}
