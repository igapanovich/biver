use crate::common::{samples, test_env};
use biver_core::operations;
use rstest::rstest;
use std::error::Error;

mod common;

#[rstest]
#[case(samples::EMPTY)]
#[case(samples::SAMPLE1)]
#[case(samples::SAMPLE2)]
#[case(samples::SAMPLE3)]
#[case(samples::SAMPLE4)]
#[case(samples::SAMPLE5)]
#[case(samples::SAMPLE6)]
#[case(samples::SAMPLE7)]
#[case(samples::SAMPLE8)]
#[case(samples::SAMPLE9)]
pub fn init_succeeds(#[case] file_content: &[u8]) -> Result<(), Box<dyn Error>> {
    let env = test_env::create_with_versioned_file(file_content)?;

    operations::init(&env.config, &env.repo_paths, None, None)?;

    assert!(operations::read_repository(&env.repo_paths)?.is_initialized());

    Ok(())
}
