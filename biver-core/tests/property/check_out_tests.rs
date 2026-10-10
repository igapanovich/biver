use crate::helpers::arb;
use crate::helpers::extensions::IndexExt;
use crate::helpers::version_path::ResolveVersionPathExt;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use biver_core::data::Head;
use biver_core::repository::{CheckOutBranchResult, CheckOutVersionResult};
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn check_out_branch_main_group(env in arb::env(), branch_index in any::<Index>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let branch_names = session.tree().branch_names().collect_vec();
        let valid_branch_name = branch_index.get_copied(&branch_names).clone();

        let result = session.check_out_branch(valid_branch_name.clone())?;

        let after = env.snapshot(&session);

        match result {
            CheckOutBranchResult::Ok => {
                prop_assert!(!before.has_uncommitted_changes, "check_out_branch succeeds only if there are no uncommitted changes");

                prop_assert!(!after.has_uncommitted_changes, "successful check_out_branch does not leave uncommitted changes");
                prop_assert_eq!(after.head, Head::Branch(valid_branch_name), "successful check_out_branch makes head point at the target branch");
                prop_assert_eq!(&before.branches, &after.branches, "successful check_out_branch does not modify branches");
                prop_assert_eq!(&before.versions, &after.versions, "successful check_out_branch does not modify versions");
            },
            CheckOutBranchResult::HasUncommittedChanges => {
                prop_assert!(before.has_uncommitted_changes, "check_out_branch fails if there are uncommitted changes");

                prop_assert_eq!(before, after, "failed check_out_branch does not modify anything");
            }
            CheckOutBranchResult::BranchNotFound => {
                prop_assert!(false, "check_out_branch does not fail because of a valid target branch");
            },
        }
    }

    #[test]
    fn check_out_version_main_group(env in arb::env(), version_path in arb::version_path()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let valid_version_id = session.tree().resolve_version_path(&version_path).id;

        let result = session.check_out_version(valid_version_id)?;

        let after = env.snapshot(&session);

        match result {
            CheckOutVersionResult::Ok => {
                prop_assert!(!before.has_uncommitted_changes, "check_out_version succeeds only if there are no uncommitted changes");

                prop_assert!(!after.has_uncommitted_changes, "successful check_out_version does not leave uncommitted changes");
                prop_assert_eq!(after.head, Head::Version(valid_version_id), "successful check_out_version makes head point at the target version");
                prop_assert_eq!(&before.branches, &after.branches, "successful check_out_version does not modify branches");
                prop_assert_eq!(&before.versions, &after.versions, "successful check_out_version does not modify versions");
            },
            CheckOutVersionResult::HasUncommittedChanges => {
                prop_assert!(before.has_uncommitted_changes, "check_out_version fails if there are uncommitted changes");

                prop_assert_eq!(before, after, "failed check_out_version does not modify anything");
            }
            CheckOutVersionResult::VersionNotFound => {
                prop_assert!(false, "check_out_version does not fail because of a valid target version");
            },
        }
    }
}

proptest! {
    #[test]
    fn check_out_branch_fails_when_target_branch_is_invalid(env in arb::env(), branch_name in arb::branch_name()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().branch_names().all(|b| b != &branch_name));

        session.discard()?;

        let before = env.snapshot(&session);

        let result = session.check_out_branch(branch_name)?;

        let after = env.snapshot(&session);

        prop_assert!(result.is_branch_not_found());
        prop_assert_eq!(before, after);
    }

    #[test]
    fn check_out_version_fails_when_target_version_is_invalid(env in arb::env(), version_id in arb::version_id()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().versions().all(|v| v.id != version_id));

        session.discard()?;

        let before = env.snapshot(&session);

        let result = session.check_out_version(version_id)?;

        let after = env.snapshot(&session);

        prop_assert!(result.is_version_not_found());
        prop_assert_eq!(before, after);
    }
}
