use crate::common::{samples, test_env};
use biver_core::operations;
use rstest::rstest;
use std::fs;

mod common;

#[rstest]
fn commit_succeeds(
    #[values(
        samples::EMPTY,
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

    fs::write(env.versioned_file_path, committed_content)?;

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
