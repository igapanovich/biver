use crate::helpers::arb;
use crate::helpers::arb::TestEnvOption::{NoUncommittedChanges, UncommittedChanges};
use crate::helpers::extensions::IndexExt;
use biver_core::data::Head;
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #[test]
    fn check_out_branch_ok_properties(env in arb::env_with_opts(&[NoUncommittedChanges]), branch_index in any::<Index>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let branch_names = session.tree().branch_names().collect_vec();
        let valid_branch_name = branch_index.get_copied(&branch_names).clone();

        let result = session.check_out_branch(valid_branch_name.clone())?;

        let after = env.snapshot(&session);

        prop_assert!(result.is_ok(), "check_out_branch succeeds if there are no uncommitted changes and target branch is valid");

        prop_assert!(!after.has_uncommitted_changes, "successful check_out_branch does not leave uncommitted changes");
        prop_assert_eq!(after.head, Head::Branch(valid_branch_name), "successful check_out_branch makes head point at the target branch");
        prop_assert_eq!(&before.branches, &after.branches, "successful check_out_branch does not modify branches");
        prop_assert_eq!(&before.versions, &after.versions, "successful check_out_branch does not modify versions");
    }

    #[test]
    fn check_out_branch_has_uncommitted_changes_properties(env in arb::env_with_opts(&[UncommittedChanges]), branch_index in any::<Index>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let branch_names = session.tree().branch_names().collect_vec();
        let valid_branch_name = branch_index.get_copied(&branch_names).clone();

        let result = session.check_out_branch(valid_branch_name.clone())?;

        let after = env.snapshot(&session);

        prop_assert!(result.is_has_uncommitted_changes(), "check_out_branch fails if there are uncommitted changes");
        prop_assert_eq!(before, after, "failed check_out_branch does not modify anything");
    }

    #[test]
    fn check_out_branch_branch_not_found_properties(env in arb::env_with_opts(&[NoUncommittedChanges]), branch_name in arb::branch_name()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().branch_names().all(|b| b != &branch_name));

        let before = env.snapshot(&session);

        let result = session.check_out_branch(branch_name)?;

        let after = env.snapshot(&session);

        prop_assert!(result.is_branch_not_found(), "check_out_branch fails if branch is not found");
        prop_assert_eq!(before, after, "failed check_out_branch does not modify anything");
    }
}
