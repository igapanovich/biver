use crate::helpers::arb;
use crate::helpers::arb::TestEnvOption::{NoUncommittedChanges, UncommittedChanges};
use crate::helpers::version_path::ResolveVersionPathExt;
use biver_core::data::Head;
use proptest::prelude::*;

proptest! {
    #[test]
    fn check_out_version_ok_properties(env in arb::env_with_opts(&[NoUncommittedChanges]), version_path in arb::version_path()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let valid_version_id = session.tree().resolve_version_path(&version_path).id;

        let result = session.check_out_version(valid_version_id)?;

        let after = env.snapshot(&session);

        prop_assert!(result.is_ok(), "check_out_version succeeds if there are no uncommitted changes and target version is valid");

        prop_assert!(!after.has_uncommitted_changes, "successful check_out_version does not leave uncommitted changes");
        prop_assert_eq!(after.head, Head::Version(valid_version_id), "successful check_out_version makes head point at the target version");
        prop_assert_eq!(&before.branches, &after.branches, "successful check_out_version does not modify branches");
        prop_assert_eq!(&before.versions, &after.versions, "successful check_out_version does not modify versions");
    }

    #[test]
    fn check_out_version_has_uncommitted_changes_properties(env in arb::env_with_opts(&[UncommittedChanges]), version_path in arb::version_path()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let valid_version_id = session.tree().resolve_version_path(&version_path).id;

        let result = session.check_out_version(valid_version_id)?;

        let after = env.snapshot(&session);

        prop_assert!(result.is_has_uncommitted_changes(), "check_out_version fails if there are uncommitted changes");
        prop_assert_eq!(before, after, "failed check_out_version does not modify anything");
    }

    #[test]
    fn check_out_version_version_not_found_properties(env in arb::env_with_opts(&[NoUncommittedChanges]), version_id in arb::version_id()) {
        let mut session = env.start_session();

        prop_assume!(session.tree().versions().all(|v| v.id != version_id));

        let before = env.snapshot(&session);

        let result = session.check_out_version(version_id)?;

        let after = env.snapshot(&session);

        prop_assert!(result.is_version_not_found(), "check_out_version fails if version is not found");
        prop_assert_eq!(before, after, "failed check_out_version does not modify anything");
    }
}
