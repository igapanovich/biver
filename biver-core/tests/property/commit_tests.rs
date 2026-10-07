use crate::helpers::arb;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use biver_core::operations;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn commit_main_group(env in arb::env()) {
        let head_before = env.repository().head;
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = env.versions_with_content();
        let had_uncommitted_changes_before = env.has_uncommitted_changes();

        let outcome = operations::commit(env.config(), env.paths(), &mut env.repository(), None)?;

        let head_after = env.repository().head;
        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = env.versions_with_content();
        let has_uncommitted_changes_after = env.has_uncommitted_changes();

        if head_before.is_branch() && had_uncommitted_changes_before {
            prop_assert!(outcome.is_ok(), "commit succeeds when head is on a branch and there are uncommitted changes");

            prop_assert_eq!(
                versions_with_content_after.len(),
                versions_with_content_before.len() + 1,
                "successful commit increases version count by one"
            );

            prop_assert!(!has_uncommitted_changes_after, "successful commit does not leave uncommitted changes");
        } else if !head_before.is_branch() {
            prop_assert!(outcome.is_head_must_be_on_branch(), "commit fails when head is not on a branch");
        } else {
            prop_assert!(outcome.is_nothing_to_commit(), "commit is noop when there are no uncommitted changes");
            prop_assert!(!has_uncommitted_changes_after, "noop commit does not create uncommitted changes");
        }

        prop_assert_eq!(versioned_file_bytes_before, versioned_file_bytes_after, "commit does not modify versioned file");

        prop_assert_eq!(
            &versions_with_content_before,
            &versions_with_content_after.intersection(&versions_with_content_before).cloned().collect(),
            "commit does not modify existing versions"
        );

        prop_assert_eq!(head_before, head_after, "commit does not modify head");
    }
}
