use crate::common::test_env::TestEnv;
use biver_core::data::Snapshot;
use biver_core::environment::Environment;
use biver_core::repository::Paths;
use itertools::Itertools;
use proptest::prelude::TestCaseError;
use proptest::{prop_assert, prop_assert_eq};
use uuid::Uuid;

pub fn assert_repository_integrity(
    test_env: &TestEnv<impl Environment + Clone>,
) -> Result<(), TestCaseError> {
    let paths = Paths::new(test_env.versioned_file_path());

    assert_full_content_dir_integrity(test_env, &paths)?;
    assert_version_content_integrity_all(test_env)?;

    Ok(())
}

fn assert_full_content_dir_integrity(
    test_env: &TestEnv<impl Environment + Clone>,
    paths: &Paths,
) -> Result<(), TestCaseError> {
    let session = test_env.start_session();
    let branch_tips = session.tree().branch_tip_map().values().copied();

    for id in branch_tips {
        prop_assert!(
            test_env
                .env()
                .file_exists(&paths.version_full_content_file(id))?
        );
    }

    Ok(())
}

fn assert_version_content_integrity_all(
    test_env: &TestEnv<impl Environment + Clone>,
) -> Result<(), TestCaseError> {
    assert_version_content_integrity(test_env, Snapshot::Main)?;
    assert_version_content_integrity(test_env, Snapshot::Snapshot1)?;
    assert_version_content_integrity(test_env, Snapshot::Snapshot2)?;
    assert_version_content_integrity(test_env, Snapshot::Snapshot3)?;
    assert_version_content_integrity(test_env, Snapshot::Snapshot4)?;
    assert_version_content_integrity(test_env, Snapshot::Snapshot5)?;
    assert_version_content_integrity(test_env, Snapshot::Snapshot6)?;
    assert_version_content_integrity(test_env, Snapshot::Snapshot7)?;
    assert_version_content_integrity(test_env, Snapshot::Snapshot8)?;

    Ok(())
}

fn assert_version_content_integrity(
    test_env: &TestEnv<impl Environment + Clone>,
    snapshot: Snapshot,
) -> Result<(), TestCaseError> {
    let session = match test_env.start_session_with_snapshot(snapshot) {
        Ok(session) => session,
        Err(biver_core::Error::SnapshotUnavailable) => return Ok(()),
        Err(e) => return Err(TestCaseError::from(e)),
    };

    let versions = session
        .tree()
        .versions()
        .map(|v| (v.id, v.xxh3_128))
        .collect_vec();

    let tmp = test_env.test_dir().join(Uuid::new_v4().to_string());

    for (version_id, expected_hash) in versions {
        session.restore_version(version_id, &tmp)?;

        let actual_hash = biver_core::xxh3_128::read(test_env.env().read_file(tmp.as_path())?)?;

        test_env.env().remove_file(tmp.as_path())?;

        prop_assert_eq!(actual_hash, expected_hash);
    }

    Ok(())
}
