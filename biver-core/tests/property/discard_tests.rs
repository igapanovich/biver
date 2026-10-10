use crate::helpers::arb;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn discard_main_group(env in arb::env()) {
        let session = env.start_session();

        let before = env.snapshot(&session);

        session.discard()?;

        let after = env.snapshot(&session);

        prop_assert!(!after.has_uncommitted_changes, "discard does not leave uncommitted changes");
        prop_assert_eq!(before.head, after.head, "discard does not modify head");
        prop_assert_eq!(before.branches, after.branches, "discard does not modify branches");
        prop_assert_eq!(before.versions, after.versions, "discard does not modify versions");
    }
}
