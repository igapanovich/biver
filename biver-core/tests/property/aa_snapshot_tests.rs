use crate::helpers::{arb, assert_repository_integrity};
use biver_core::repository::Paths;
use proptest::prelude::*;
use std::time::Duration;

proptest! {
    #[test]
    fn snapshot_properties(mut test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        let paths = Paths::new(test_env.versioned_file_path());
        let mut session = test_env.start_session();

        for op in operations {
            test_env.advance_time(Duration::from_hours(3));
            op.execute(&paths, &mut session)?;
        }

        drop(session);
        assert_repository_integrity(&test_env)?;
    }
}
