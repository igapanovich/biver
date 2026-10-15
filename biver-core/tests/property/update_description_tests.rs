use crate::helpers::diff::Diff;
use crate::helpers::snapshot::Snapshot;
use crate::helpers::version_path::ResolveVersionPathExt;
use crate::helpers::{arb, assert_version_content_integrity};
use proptest::prelude::*;

proptest! {
    #[test]
    fn update_description_properties(env in arb::env(), target in arb::version_path(), new_description in any::<String>()) {
        let mut session = env.start_session();

        let before = env.snapshot(&session);

        let target = session.tree().resolve_version_path(&target).id;
        session.update_description(target, new_description)?;

        let after = env.snapshot(&session);

        prop_assert_eq!(&before.versioned_file_content, &after.versioned_file_content, "update_description does not modify versioned file");
        prop_assert_eq!(&before.head, &after.head, "update_description does not modify head");
        prop_assert_eq!(&before.branches, &after.branches, "update_description does not modify branches");

        let diff = Snapshot::diff(&before, &after);

        prop_assert!(diff.inserted_versions.len() <= 1, "update_description modifies at most one version");
        prop_assert_eq!(diff.inserted_versions.len(), diff.removed_versions.len());

        if diff.inserted_versions.len() == 1 {
            let before = diff.removed_versions.iter().last().unwrap();
            let after = diff.inserted_versions.iter().last().unwrap();
            prop_assert_eq!(&before.id, &after.id, "update_description does not modify version ID");
            prop_assert_eq!(&before.parent_id, &after.parent_id, "update_description does not modify version parent");
            prop_assert_eq!(&before.xxh3_128, &after.xxh3_128, "update_description does not modify version file content");
        }

        assert_version_content_integrity(session)?;
    }
}
