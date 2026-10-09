use crate::helpers::arb;
use crate::helpers::extensions::SessionExt;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn discard_main_group(env in arb::env()) {
        let session = env.start_session();

        let head_before = session.tree().head().clone();
        let branches_before = session.tree().branch_tip_ids().clone();
        let versions_with_content_before = session.versions_with_content();

        session.discard()?;

        let head_after = session.tree().head().clone();
        let branches_after = session.tree().branch_tip_ids().clone();
        let versions_with_content_after = session.versions_with_content();

        prop_assert_eq!(session.has_uncommitted_changes()?, false, "discard does not leave uncommitted changes");

        prop_assert_eq!(head_before, head_after, "discard does not modify head");

        prop_assert_eq!(branches_before, branches_after, "discard does not modify branches");

        prop_assert_eq!(versions_with_content_before, versions_with_content_after, "discard does not modify versions");
    }
}
