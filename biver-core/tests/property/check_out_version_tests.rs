use crate::helpers::version_path::ResolveVersionPathExt;
use crate::helpers::{arb, assert_repository_integrity};
use biver_core::data::Head;
use biver_core::repository::CheckOutVersionResult;
use proptest::prelude::*;

proptest! {
    #[test]
    fn check_out_version_ok_properties(env in arb::env(), version_path in arb::version_path()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let valid_version_id = session.tree().resolve_version_path(&version_path).id;

        let result = session.check_out_version(valid_version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CheckOutVersionResult::Ok, "check_out_version succeeds if target version is valid");

        if before.has_uncommitted_changes {
            prop_assert_eq!(before.versioned_file_content, after.versioned_file_content, "successful check_out_version does not modify versioned file if there were uncommitted changes");
        } else {
            prop_assert!(!after.has_uncommitted_changes, "successful check_out_version restores versioned file to the target version if there were no uncommitted changes");
        }

        prop_assert_eq!(after.head, Head::Version(valid_version_id), "successful check_out_version makes head point at the target version");
        prop_assert_eq!(&before.branches, &after.branches, "successful check_out_version does not modify branches");
        prop_assert_eq!(&before.versions, &after.versions, "successful check_out_version does not modify versions");

        assert_repository_integrity(session)?;
    }

    #[test]
    fn check_out_version_version_not_found_properties(env in arb::env(), version_id in arb::version_id()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().versions().all(|v| v.id != version_id));

        let before = env.snapshot(&session);

        let result = session.check_out_version(version_id)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(result, CheckOutVersionResult::VersionNotFound, "check_out_version fails if version is not found");

        prop_assert_eq!(before, after, "failed check_out_version does not modify anything");

        assert_repository_integrity(session)?;
    }
}
