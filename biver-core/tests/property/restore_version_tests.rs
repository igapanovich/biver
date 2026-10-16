use crate::helpers::arb::TestEnvOption::{NoUncommittedChanges, UncommittedChanges};
use crate::helpers::version_path::ResolveVersionPathExt;
use crate::helpers::{arb, assert_repository_integrity};
use biver_core::repository::{Paths, RestoreVersionResult};
use proptest::option;
use proptest::prelude::*;

proptest! {
    #[test]
    fn restore_version_into_versioned_file_ok_properties(env in arb::env_with_opts(&[NoUncommittedChanges]), version_path in arb::version_path()) {
        let session = env.start_session();

        let valid_version_id = session.tree().resolve_version_path(&version_path).id;

        let before = env.snapshot(&session);

        let result = session.restore_version(valid_version_id, env.versioned_file_path())?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RestoreVersionResult::Ok, "restore_version succeeds if there are no uncommitted changes and target version is valid");

        prop_assert_eq!(&before.versions, &after.versions, "successful restore_version does not modify versions");
        prop_assert_eq!(&before.head, &after.head, "successful restore_version does not modify head");
        prop_assert_eq!(&before.branches, &after.branches, "successful restore_version does not modify branches");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn restore_version_into_versioned_file_version_not_found_properties(env in arb::env_with_opts(&[NoUncommittedChanges]), version_id in arb::version_id()) {
        let session = env.start_session();

        prop_assume!(session.tree().version(version_id).is_none());

        let before = env.snapshot(&session);

        let result = session.restore_version(version_id, env.versioned_file_path())?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RestoreVersionResult::VersionNotFound, "restore_version fails if the target version was not found");

        prop_assert_eq!(before, after, "failed restore_version does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn restore_version_into_versioned_file_would_overwrite_uncommitted_changes_properties(env in arb::env_with_opts(&[UncommittedChanges]), version_path in arb::version_path()) {
        let session = env.start_session();

        let valid_version_id = session.tree().resolve_version_path(&version_path).id;

        let before = env.snapshot(&session);

        let result = session.restore_version(valid_version_id, env.versioned_file_path())?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RestoreVersionResult::WouldOverwriteUncommittedChanges, "restore_version fails if there are uncommitted changes");

        prop_assert_eq!(before, after, "failed restore_version does not modify anything");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn restore_version_into_repository_properties(env in arb::env_with_opts(&[NoUncommittedChanges]), version_path in arb::version_path(), sub_path in option::of(arb::fs_path())) {
        let session = env.start_session();

        println!("{:?}", sub_path);

        let paths = Paths::new(env.versioned_file_path());

        let target_path = if let Some(sub_path) = sub_path {
            paths.repository_dir().join(sub_path)
        } else {
            paths.repository_dir().to_path_buf()
        };

        let valid_version_id = session.tree().resolve_version_path(&version_path).id;

        let before = env.snapshot(&session);

        let result = session.restore_version(valid_version_id, &target_path)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RestoreVersionResult::CannotRestoreIntoRepositoryDir, "restore_version fails if target path is inside the repository");

        prop_assert_eq!(before, after, "failed restore_version does not modify anything");

        assert_repository_integrity(session)?;
    }

    fn restore_version_into_another_file_ok_properties(env in arb::env(), version_path in arb::version_path(), sub_path in arb::fs_path()) {
        let session = env.start_session();

        let target_path = env.test_dir().join(&sub_path);

        let paths = Paths::new(env.versioned_file_path());

        prop_assume!(target_path != env.versioned_file_path());
        prop_assume!(!target_path.starts_with(paths.repository_dir()));

        let valid_version_id = session.tree().resolve_version_path(&version_path).id;

        let before = env.snapshot(&session);

        let result = session.restore_version(valid_version_id, env.versioned_file_path())?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, RestoreVersionResult::Ok, "restore_version succeeds if target file is not the versioned file and target version is valid");

        prop_assert_eq!(&before.versions, &after.versions, "successful restore_version does not modify versions");
        prop_assert_eq!(&before.head, &after.head, "successful restore_version does not modify head");
        prop_assert_eq!(&before.branches, &after.branches, "successful restore_version does not modify branches");

        assert_repository_integrity(session)?;
    }
}
