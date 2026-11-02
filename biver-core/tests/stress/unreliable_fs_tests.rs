use crate::common::repository_operation::RepositoryOperation;
use crate::common::test_env::TestEnv;
use crate::common::time_control_environment::TimeControlEnvironment;
use crate::common::unreliable_fs_environment::UnreliableFsEnvironment;
use crate::common::{arb, assert_repository_integrity};
use biver_core::repository::Paths;
use proptest::prelude::*;
use std::time::Duration;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn unreliable_fs_test_instance1(test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        unreliable_fs_test(test_env, operations)?;
    }

    #[test]
    fn unreliable_fs_test_instance2(test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        unreliable_fs_test(test_env, operations)?;
    }

    #[test]
    fn unreliable_fs_test_instance3(test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        unreliable_fs_test(test_env, operations)?;
    }

    #[test]
    fn unreliable_fs_test_instance4(test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        unreliable_fs_test(test_env, operations)?;
    }

    #[test]
    fn unreliable_fs_test_instance5(test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        unreliable_fs_test(test_env, operations)?;
    }

    #[test]
    fn unreliable_fs_test_instance6(test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        unreliable_fs_test(test_env, operations)?;
    }

    #[test]
    fn unreliable_fs_test_instance7(test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        unreliable_fs_test(test_env, operations)?;
    }

    #[test]
    fn unreliable_fs_test_instance8(test_env in arb::test_env_with_time_control(), operations in arb::repository_operation::operations()) {
        unreliable_fs_test(test_env, operations)?;
    }
}

fn unreliable_fs_test(
    test_env: TestEnv<TimeControlEnvironment>,
    operations: Vec<RepositoryOperation>,
) -> Result<(), TestCaseError> {
    let paths = Paths::new(test_env.versioned_file_path());

    let mut unreliable_test_env =
        test_env.clone_with_env(|env| UnreliableFsEnvironment::new(env.clone()));

    for op in operations {
        unreliable_test_env
            .env_mut()
            .inner_mut()
            .advance_time(Duration::from_hours(3));

        let Ok(mut session) = unreliable_test_env.try_start_session() else {
            continue;
        };

        let _ = op.execute(&paths, &mut session);

        drop(session);
        assert_repository_integrity(&test_env)?;
    }

    Ok(())
}
