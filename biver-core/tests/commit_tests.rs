use crate::common::{samples, test_env};
use biver_core::operations;
use rstest::rstest;
use std::fs;

mod common;
mod property_based;

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
    let mut env = test_env::create_initialized(initial_content)?;

    fs::write(&env.versioned_file_path, committed_content)?;

    let commit_outcome = operations::commit(&env.config, &env.repo_paths, &mut env.repo, None)?;

    if initial_content.eq(committed_content) {
        assert!(commit_outcome.is_nothing_to_commit())
    } else {
        assert!(commit_outcome.is_ok());
    }

    let has_uncommitted_changes =
        operations::has_uncommitted_changes(&env.repo_paths, &mut env.repo)?;

    assert!(!has_uncommitted_changes);

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
    let mut env = test_env::create_initialized(samples::EMPTY)?;

    fs::write(&env.versioned_file_path, committed_content)?;

    let commit_outcome = operations::commit(&env.config, &env.repo_paths, &mut env.repo, None)?;

    assert!(commit_outcome.is_ok());

    let has_uncommitted_changes =
        operations::has_uncommitted_changes(&env.repo_paths, &mut env.repo)?;

    assert!(!has_uncommitted_changes);

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
    let mut env = test_env::create_initialized(initial_content)?;

    fs::write(&env.versioned_file_path, samples::EMPTY)?;

    let commit_outcome = operations::commit(&env.config, &env.repo_paths, &mut env.repo, None)?;

    assert!(commit_outcome.is_ok());

    let has_uncommitted_changes =
        operations::has_uncommitted_changes(&env.repo_paths, &mut env.repo)?;

    assert!(!has_uncommitted_changes);

    Ok(())
}
