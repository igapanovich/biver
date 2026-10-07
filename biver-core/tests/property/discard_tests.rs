use crate::helpers::arb;
use biver_core::operations;
use proptest::prelude::*;

proptest! {
    #[test]
    fn discard_does_not_leave_uncommitted_changes(env in arb::env()) {
        operations::discard(env.config(), env.paths(), &mut env.repository()).unwrap();

        prop_assert!(!env.has_uncommitted_changes(), "discard does not leave uncommitted changes");
    }
}
