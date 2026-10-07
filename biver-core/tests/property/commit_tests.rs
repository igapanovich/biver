use crate::helpers::arb;
use biver_core::operations;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256 * 5))]

    #[test]
    fn commit_main_group(env in arb::env()) {
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = env.versions_with_content();

        let had_uncommitted_changes_before = env.has_uncommitted_changes();

        let outcome = operations::commit(env.config(), env.paths(), &mut env.repository(), None)?;

        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = env.versions_with_content();

        if had_uncommitted_changes_before {
            prop_assert!(outcome.is_ok(), "commit succeeds when there are uncommitted changes");

            prop_assert_eq!(
                versions_with_content_after.len(),
                versions_with_content_before.len() + 1,
                "successful commit increases version count by one"
            );
        } else {
            prop_assert!(outcome.is_nothing_to_commit(), "commit is noop when there are no uncommitted changes");
        }

        prop_assert!(!env.has_uncommitted_changes(), "commit does not leave uncommitted changes");

        prop_assert_eq!(versioned_file_bytes_before, versioned_file_bytes_after, "commit does not modify versioned file");

        prop_assert_eq!(
            &versions_with_content_before,
            &versions_with_content_after.intersection(&versions_with_content_before).cloned().collect(),
            "commit does not modify existing versions"
        );
    }
}
