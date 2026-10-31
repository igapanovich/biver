use crate::helpers::{arb, assert_repository_integrity};
use proptest::prelude::*;

proptest! {
    #[test]
    fn tidy_up_properties(test_env in arb::test_env()) {
        let session = test_env.start_session();

        let before = test_env.snapshot(&session);

        let result = session.tidy_up();

        let after = test_env.snapshot(&session);

        prop_assert!(result.is_ok(), "tidy_up always succeeds");

        prop_assert_eq!(before, after, "tidy_up does not modify live data");

        drop(session);
        assert_repository_integrity(test_env)?;
    }
}
