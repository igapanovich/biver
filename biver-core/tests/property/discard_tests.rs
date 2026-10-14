use crate::helpers::{arb, assert_version_content_integrity};
use proptest::prelude::*;

proptest! {
    #[test]
    fn discard_properties(env in arb::env()) {
        let session = env.start_session();

        let before = env.snapshot(&session);

        session.discard()?;

        let after = env.snapshot(&session);

        prop_assert!(!after.has_uncommitted_changes, "discard does not leave uncommitted changes");
        prop_assert_eq!(before.head, after.head, "discard does not modify head");
        prop_assert_eq!(before.branches, after.branches, "discard does not modify branches");
        prop_assert_eq!(before.versions, after.versions, "discard does not modify versions");

        assert_version_content_integrity(session)?;
    }
}
