use crate::helpers::{arb, assert_repository_integrity};
use proptest::prelude::*;

proptest! {
    #[test]
    fn discard_properties(test_env in arb::test_env()) {
        let session = test_env.start_session();

        let before = test_env.snapshot(&session);

        session.discard()?;

        let after = test_env.snapshot(&session);

        prop_assert!(!after.has_uncommitted_changes, "discard does not leave uncommitted changes");
        prop_assert_eq!(before.head, after.head, "discard does not modify head");
        prop_assert_eq!(before.branches, after.branches, "discard does not modify branches");
        prop_assert_eq!(before.versions, after.versions, "discard does not modify versions");

        drop(session);
        assert_repository_integrity(&test_env)?;
    }
}
