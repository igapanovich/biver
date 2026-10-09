use crate::helpers::{samples, test_env};
use biver_core::repository;
use rstest::rstest;
use std::error::Error;
use std::fs;

#[rstest]
#[case(samples::EMPTY)]
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
pub fn initialize_succeeds(#[case] file_content: &[u8]) -> Result<(), Box<dyn Error>> {
    let env = test_env::create_with_versioned_file(file_content)?;

    let init_ok = repository::initialize(
        env.config.clone(),
        env.versioned_file_path.clone(),
        None,
        None,
    )?;

    assert!(!init_ok.was_already_initialized);

    Ok(())
}

#[rstest]
#[case(samples::EMPTY)]
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
pub fn initialize_does_not_modify_versioned_file(
    #[case] file_content: &[u8],
) -> Result<(), Box<dyn Error>> {
    let env = test_env::create_with_versioned_file(file_content)?;

    let init_ok = repository::initialize(
        env.config.clone(),
        env.versioned_file_path.clone(),
        None,
        None,
    )?;

    let file_content_after_init = fs::read(&env.versioned_file_path)?;

    assert_eq!(file_content, file_content_after_init);

    Ok(())
}
