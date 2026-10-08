use crate::helpers::arb;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use biver_core::operations;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn discard_main_group(env in arb::env()) {
        let head_before = env.repository().head;
        let branches_before = env.repository().branches;
        let versions_with_content_before = env.versions_with_content();

        operations::discard(env.config(), env.paths(), &mut env.repository()).unwrap();

        let head_after = env.repository().head;
        let branches_after = env.repository().branches;
        let versions_with_content_after = env.versions_with_content();

        prop_assert!(!env.has_uncommitted_changes(), "discard does not leave uncommitted changes");

        prop_assert_eq!(head_before, head_after, "discard does not modify head");

        prop_assert_eq!(branches_before, branches_after, "discard does not modify branches");

        prop_assert_eq!(versions_with_content_before, versions_with_content_after, "discard does not modify versions");
    }
}
