use crate::helpers::extensions::UnwrapTryStartSessionExt;
use crate::helpers::{samples, test_env};
use biver_core::repository;
use rstest::rstest;
use std::fs;

#[rstest]
fn commit_succeeds(
    #[values(
        samples::EMPTY,
        samples::SAMPLE0,
        samples::SAMPLE1,
        samples::SAMPLE2,
        samples::SAMPLE3,
        samples::SAMPLE4,
        samples::SAMPLE5,
        samples::SAMPLE6,
        samples::SAMPLE7,
        samples::SAMPLE8,
        samples::SAMPLE9
    )]
    initial_content: &[u8],
    #[values(
        samples::EMPTY,
        samples::SAMPLE0,
        samples::SAMPLE1,
        samples::SAMPLE2,
        samples::SAMPLE3,
        samples::SAMPLE4,
        samples::SAMPLE5,
        samples::SAMPLE6,
        samples::SAMPLE7,
        samples::SAMPLE8,
        samples::SAMPLE9
    )]
    committed_content: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let env = test_env::create_initialized(initial_content)?;

    fs::write(&env.versioned_file_path, committed_content)?;

    let mut session =
        repository::try_start_session(env.config.clone(), env.versioned_file_path.clone())?
            .unwrap();

    let commit_result = session.commit(None)?;

    if initial_content.eq(committed_content) {
        assert!(commit_result.is_no_uncommitted_changes())
    } else {
        assert!(commit_result.is_ok());
    }

    assert_eq!(session.has_uncommitted_changes()?, false);

    Ok(())
}

#[rstest]
#[case(samples::SAMPLE0)]
#[case(samples::SAMPLE1)]
#[case(samples::SAMPLE2)]
#[case(samples::SAMPLE3)]
#[case(samples::SAMPLE4)]
#[case(samples::SAMPLE5)]
#[case(samples::SAMPLE6)]
#[case(samples::SAMPLE7)]
#[case(samples::SAMPLE8)]
#[case(samples::SAMPLE9)]
fn commit_from_empty_to_nonempty_succeeds(
    #[case] committed_content: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let env = test_env::create_initialized(samples::EMPTY)?;

    fs::write(&env.versioned_file_path, committed_content)?;

    let mut session =
        repository::try_start_session(env.config.clone(), env.versioned_file_path.clone())?
            .unwrap();

    let commit_result = session.commit(None)?;

    assert!(commit_result.is_ok());

    assert_eq!(session.has_uncommitted_changes()?, false);

    Ok(())
}

#[rstest]
#[case(samples::SAMPLE0)]
#[case(samples::SAMPLE1)]
#[case(samples::SAMPLE2)]
#[case(samples::SAMPLE3)]
#[case(samples::SAMPLE4)]
#[case(samples::SAMPLE5)]
#[case(samples::SAMPLE6)]
#[case(samples::SAMPLE7)]
#[case(samples::SAMPLE8)]
#[case(samples::SAMPLE9)]
fn commit_from_nonempty_to_empty_succeeds(
    #[case] initial_content: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let env = test_env::create_initialized(initial_content)?;

    fs::write(&env.versioned_file_path, samples::EMPTY)?;

    let mut session =
        repository::try_start_session(env.config.clone(), env.versioned_file_path.clone())?
            .unwrap();

    let commit_result = session.commit(None)?;

    assert!(commit_result.is_ok());

    assert!(commit_result.is_ok());

    assert_eq!(session.has_uncommitted_changes()?, false);

    Ok(())
}
