use crate::helpers::arb;
use crate::helpers::diff::Diff;
use crate::helpers::extensions::IteratorExt;
use crate::helpers::snapshot::Snapshot;
use crate::helpers::version_path::ResolveVersionPathExt;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn update_description_main_group(env in arb::env(), target in arb::version_path(), new_description in any::<String>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let target = session.tree().resolve_version_path(&target).id;
        session.update_description(target, new_description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "update_description does not modify versioned file");
        prop_assert_eq!(&before.head, &after.head, "update_description does not modify head");
        prop_assert_eq!(&before.branches, &after.branches, "update_description does not modify branches");

        let version_contents_before = before.versions.iter().map(|v| (v.version.id, &v.content_blob)).collect_set();
        let version_contents_after = after.versions.iter().map(|v| (v.version.id, &v.content_blob)).collect_set();
        prop_assert_eq!(version_contents_before, version_contents_after, "update_description does not modify version contents");

        let diff = Snapshot::diff(&before, &after);

        prop_assert!(diff.removed_versions.len() <= 1, "update_description modifies at most one version");
        prop_assert!(diff.inserted_versions.len() <= 1, "update_description modifies at most one version");
    }
}
