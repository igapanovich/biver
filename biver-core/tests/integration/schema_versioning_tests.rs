use crate::helpers::test_env::TestEnv;
use biver_core::repository::Paths;
use biver_std_environment::StdEnvironment;
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
    let test_env = TestEnv::new(StdEnvironment);

    let paths = Paths::new(&test_env.versioned_file_path());

    fs::write(paths.schema_version_file(), schema_version)?;

    let session_error = test_env.start_session().err();

    assert_matches!(
        session_error,
        Some(biver_core::Error::IncompatibleSchemaVersion(error_ver)) if error_ver == schema_version
    );

    Ok(())
}

#[rstest]
fn try_start_session_fails_if_schema_version_missing() -> Result<(), Box<dyn Error>> {
    let test_env = TestEnv::new(StdEnvironment);

    let paths = Paths::new(&test_env.versioned_file_path());

    fs::remove_file(paths.schema_version_file())?;

    let session_result = test_env.start_session().err();

    assert_matches!(
        session_result,
        Some(biver_core::Error::IncompatibleSchemaVersion(error_ver)) if error_ver == ""
    );

    Ok(())
}
