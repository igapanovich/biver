use crate::helpers::arb;
use crate::helpers::checkout_target::ResolveCheckoutTargetExtension;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use biver_core::operations;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn check_out_main_group(env in arb::env(), checkout_target in arb::checkout_target()) {
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = env.versions_with_content();
        let had_uncommitted_changes_before = env.has_uncommitted_changes();

        let checkout_target = env.repository().resolve_checkout_target(&checkout_target);

        let outcome = operations::check_out(&env.config(), &env.paths(), &mut env.repository(), &checkout_target)?;

        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = env.versions_with_content();

        if had_uncommitted_changes_before {
            prop_assert!(outcome.is_has_uncommitted_changes(), "check_out fails when there are uncommitted changes");

            prop_assert_eq!(versioned_file_bytes_before, versioned_file_bytes_after, "failed check_out does not modify versioned file")
        } else {
            prop_assert!(outcome.is_ok(), "check_out succeeds when there are no uncommitted changes");

            prop_assert!(!env.has_uncommitted_changes(), "successful check_out does not leave uncommitted changes");
        }

        prop_assert_eq!(versions_with_content_before, versions_with_content_after, "check_out does not modify versions")
    }
}
