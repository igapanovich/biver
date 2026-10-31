use crate::helpers::test_env::TestEnv;
use biver_std_environment::StdEnvironment;
use rstest::rstest;
use std::assert_matches;
use std::error::Error;

#[rstest]
fn can_not_start_parallel_sessions() -> Result<(), Box<dyn Error>> {
    let test_env = TestEnv::new(StdEnvironment);

    let _session1 = test_env.start_session()?;

    assert_matches!(
        test_env.start_session().err(),
        Some(biver_core::Error::RepositoryLocked)
    );

    Ok(())
}

#[rstest]
fn can_start_sessions_sequentially() -> Result<(), Box<dyn Error>> {
    let test_env = TestEnv::new(StdEnvironment);

    let session1 = test_env.start_session()?;

    drop(session1);

    assert!(test_env.start_session().is_ok());

    Ok(())
}
