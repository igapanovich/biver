use crate::helpers::arb;
use crate::helpers::extensions::SessionExt;
use crate::helpers::version_path::ResolveVersionPathExt;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use itertools::Itertools;
use proptest::prelude::*;
use proptest::sample::Index;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn check_out_branch_main_group(env in arb::env(), branch_index in any::<Index>()) {
        let mut session = env.start_session();

        let head_before = session.tree().head().clone();
        let branches_before = session.tree().branch_tip_ids().clone();
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = session.versions_with_content();
        let had_uncommitted_changes_before = session.has_uncommitted_changes()?;

        let branch_name = *branch_index.get(&session.tree().branch_names().collect_vec());

        let outcome = session.check_out_branch(branch_name.clone())?;

        let head_after = session.tree().head().clone();
        let branches_after = session.tree().branch_tip_ids().clone();
        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = session.versions_with_content();
        let has_uncommitted_changes_after = session.has_uncommitted_changes()?;

        if had_uncommitted_changes_before {
            prop_assert!(outcome.is_has_uncommitted_changes(), "check_out_branch fails when there are uncommitted changes");

            prop_assert_eq!(versioned_file_bytes_before, versioned_file_bytes_after, "failed check_out_branch does not modify versioned file");

            prop_assert_eq!(head_before, head_after, "failed check_out_branch does not modify head");
        } else {
            prop_assert!(outcome.is_ok(), "check_out_branch succeeds when there are no uncommitted changes");

            prop_assert!(!has_uncommitted_changes_after, "successful check_out_branch does not leave uncommitted changes");

            prop_assert!(head_after.is_branch(), "successful check_out_branch makes head target a branch");
        }

        prop_assert_eq!(branches_before, branches_after, "check_out_branch does not modify branches");

        prop_assert_eq!(versions_with_content_before, versions_with_content_after, "check_out_branch does not modify versions")
    }

    #[test]
    fn check_out_version_main_group(env in arb::env(), version_path in arb::version_path()) {
        let mut session = env.start_session();

        let head_before = session.tree().head().clone();
        let branches_before = session.tree().branch_tip_ids().clone();
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = session.versions_with_content();
        let had_uncommitted_changes_before = session.has_uncommitted_changes()?;

        let version_id = session.tree().resolve_version_path(&version_path).id;

        let outcome = session.check_out_version(version_id)?;

        let head_after = session.tree().head().clone();
        let branches_after = session.tree().branch_tip_ids().clone();
        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = session.versions_with_content();
        let has_uncommitted_changes_after = session.has_uncommitted_changes()?;

        if had_uncommitted_changes_before {
            prop_assert!(outcome.is_has_uncommitted_changes(), "check_out_version fails when there are uncommitted changes");

            prop_assert_eq!(versioned_file_bytes_before, versioned_file_bytes_after, "failed check_out_version does not modify versioned file");

            prop_assert_eq!(head_before, head_after, "failed check_out_version does not modify head");
        } else {
            prop_assert!(outcome.is_ok(), "check_out_version succeeds when there are no uncommitted changes");

            prop_assert!(!has_uncommitted_changes_after, "successful check_out_version does not leave uncommitted changes");

            prop_assert!(head_after.is_version(), "successful check_out_version makes head target a version");
        }

        prop_assert_eq!(branches_before, branches_after, "check_out_version does not modify branches");

        prop_assert_eq!(versions_with_content_before, versions_with_content_after, "check_out_version does not modify versions")
    }
}
