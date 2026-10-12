use crate::helpers::{samples, test_env};
use biver_core::repository;
use rstest::rstest;
use std::assert_matches;
use std::error::Error;

#[rstest]
fn can_not_start_parallel_sessions() -> Result<(), Box<dyn Error>> {
    let env = test_env::create_initialized(samples::SAMPLE1)?;

    let _session1 = repository::try_start_session(&env.versioned_file_path)?;

    let session2_result = repository::try_start_session(&env.versioned_file_path);

    assert_matches!(session2_result, Err(biver_core::Error::RepositoryLocked));

    Ok(())
}

#[rstest]
fn can_start_sessions_sequentially() -> Result<(), Box<dyn Error>> {
    let env = test_env::create_initialized(samples::SAMPLE1)?;

    let session1 = repository::try_start_session(&env.versioned_file_path)?;

    drop(session1);

    let session2_result = repository::try_start_session(&env.versioned_file_path);

    assert_matches!(session2_result, Ok(_));

    Ok(())
}
