use crate::helpers::arb;
use crate::helpers::difference::Difference;
use crate::helpers::extensions::SessionExt;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use itertools::Itertools;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn commit_main_group(env in arb::env()) {
        let mut session = env.start_session();

        let head_before = session.tree().head().clone();
        let branches_before = session.tree().branch_tip_ids().clone();
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = session.versions_with_content();
        let had_uncommitted_changes_before = session.has_uncommitted_changes()?;

        let result = session.commit(None)?;

        let head_after = session.tree().head().clone();
        let branches_after = session.tree().branch_tip_ids().clone();
        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = session.versions_with_content();
        let has_uncommitted_changes_after = session.has_uncommitted_changes()?;

        if head_before.is_branch() && had_uncommitted_changes_before {
            prop_assert!(result.is_ok(), "commit succeeds when head is on a branch and there are uncommitted changes");

            prop_assert_eq!(
                versions_with_content_after.len(),
                versions_with_content_before.len() + 1,
                "successful commit increases version count by one"
            );

            prop_assert!(!has_uncommitted_changes_after, "successful commit does not leave uncommitted changes");
        } else if !head_before.is_branch() {
            prop_assert!(result.is_head_must_be_on_branch(), "commit fails when head is not on a branch");
        } else {
            prop_assert!(result.is_no_uncommitted_changes(), "commit is noop when there are no uncommitted changes");
            prop_assert!(!has_uncommitted_changes_after, "noop commit does not create uncommitted changes");
        }

        prop_assert_eq!(versioned_file_bytes_before, versioned_file_bytes_after, "commit does not modify versioned file");

        prop_assert_eq!(
            &versions_with_content_before,
            &versions_with_content_after.intersection(&versions_with_content_before).cloned().collect(),
            "commit does not modify existing versions"
        );

        prop_assert_eq!(&head_before, &head_after, "commit does not modify head");

        let modified_branches = branches_before.difference(&branches_after).keys().map(|n| Some(*n)).collect_vec();

        if result.is_ok() {
            prop_assert_eq!(modified_branches, vec![head_before.branch()], "successful commit modifies head branch and does not modify any other branches");
        } else {
            prop_assert_eq!(modified_branches, Vec::new(), "failed commit does not modify branches");
        }
    }
}
