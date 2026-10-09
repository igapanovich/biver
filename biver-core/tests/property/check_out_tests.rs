use crate::helpers::arb;
use crate::helpers::checkout_target::ResolveCheckoutTargetExt;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use biver_core::operations;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn check_out_main_group(env in arb::env(), checkout_target in arb::checkout_target()) {
        let head_before = env.repository().head;
        let branches_before = env.repository().branches;
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = env.versions_with_content();
        let had_uncommitted_changes_before = env.has_uncommitted_changes();

        let checkout_target_str = env.repository().resolve_checkout_target(&checkout_target);

        let outcome = operations::check_out(env.config(), env.paths(), &mut env.repository(), &checkout_target_str)?;

        let head_after = env.repository().head;
        let branches_after = env.repository().branches;
        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = env.versions_with_content();
        let has_uncommitted_changes_after = env.has_uncommitted_changes();

        if had_uncommitted_changes_before {
            prop_assert!(outcome.is_has_uncommitted_changes(), "check_out fails when there are uncommitted changes");

            prop_assert_eq!(versioned_file_bytes_before, versioned_file_bytes_after, "failed check_out does not modify versioned file");

            prop_assert_eq!(head_before, head_after, "failed check_out does not modify head");
        } else {
            prop_assert!(outcome.is_ok(), "check_out succeeds when there are no uncommitted changes");

            prop_assert!(!has_uncommitted_changes_after, "successful check_out does not leave uncommitted changes");

            if checkout_target.is_branch() {
                prop_assert!(head_after.is_branch(), "successful branch checkout makes head target a branch");
            }

            if checkout_target.is_version() {
                prop_assert!(head_after.is_version(), "successful version checkout makes head target a version");
            }
        }

        prop_assert_eq!(branches_before, branches_after, "check_out does not modify branches");

        prop_assert_eq!(versions_with_content_before, versions_with_content_after, "check_out does not modify versions")
    }
}
