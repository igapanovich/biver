use crate::helpers::arb;
use crate::helpers::diff::Diff;
use crate::helpers::snapshot::Snapshot;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use biver_core::repository::CommitResult;
use itertools::Itertools;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn commit_main_group(env in arb::env()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let result = session.commit(None)?;

        let after = env.snapshot(&session);

        match result {
            CommitResult::Ok => {
                prop_assert!(before.head.is_branch(), "commit succeeds only if head is on a branch");
                prop_assert!(before.has_uncommitted_changes, "commit succeeds only if there are uncommitted changes");

                prop_assert!(!after.has_uncommitted_changes, "successful commit does not leave uncommitted changes");
                prop_assert_eq!(&before.head, &after.head, "successful commit does not modify head");
                prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "successful commit does not modify versioned file");

                let diff = Snapshot::diff(&before, &after);

                prop_assert_eq!(diff.inserted_versions.len(), 1, "successful commit adds one versions");
                prop_assert!(diff.removed_versions.is_empty(), "successful commit does not remove versions");

                prop_assert!(diff.removed_branches.is_empty(), "successful commit does not remove branches");
                prop_assert_eq!(
                    diff.upserted_branches.keys().collect_vec(),
                    vec![before.head.branch().unwrap()],
                    "successful commit modifies head branch and only it"
                );
            }
            CommitResult::NoUncommittedChanges => {
                prop_assert!(result.is_no_uncommitted_changes(), "commit is noop if there are no uncommitted changes");

                prop_assert_eq!(before, after, "noop commit does not modify anything");
            }
            CommitResult::HeadMustBeOnBranch => {
                prop_assert!(!before.head.is_branch(), "commit fails if head is not on branch");

                prop_assert_eq!(before, after, "failed commit does not modify anything");
            }
        }
    }
}
