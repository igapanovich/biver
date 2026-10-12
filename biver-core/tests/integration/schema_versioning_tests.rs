use crate::helpers::{samples, test_env};
use biver_core::repository;
use biver_core::repository::RepositoryPaths;
use rstest::rstest;
use std::assert_matches;
use std::error::Error;
use std::fs;

#[rstest]
#[case("")]
#[case("-1")]
#[case("0")]
#[case("9999999")]
fn try_start_session_fails_if_schema_version_incompatible(
    #[case] schema_version: &str,
) -> Result<(), Box<dyn Error>> {
    let env = test_env::create_initialized(samples::SAMPLE1)?;

    let paths = RepositoryPaths::new(&env.versioned_file_path);

    fs::write(paths.schema_version_file(), schema_version)?;

    let session_result = repository::try_start_session(&env.versioned_file_path);

    assert_matches!(
        session_result,
        Err(biver_core::Error::IncompatibleSchemaVersion(error_ver)) if error_ver == schema_version
    );

    Ok(())
}

#[rstest]
fn try_start_session_fails_if_schema_version_missing() -> Result<(), Box<dyn Error>> {
    let env = test_env::create_initialized(samples::SAMPLE1)?;

    let paths = RepositoryPaths::new(&env.versioned_file_path);

    fs::remove_file(paths.schema_version_file())?;

    let session_result = repository::try_start_session(&env.versioned_file_path);

    assert_matches!(
        session_result,
        Err(biver_core::Error::IncompatibleSchemaVersion(error_ver)) if error_ver == ""
    );

    Ok(())
}
