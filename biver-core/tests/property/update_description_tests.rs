use crate::helpers::arb;
use crate::helpers::extensions::SessionExt;
use crate::helpers::version_path::ResolveVersionPathExt;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use proptest::prelude::*;
use std::collections::HashSet;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn update_description_main_group(env in arb::env(), target in arb::version_path(), new_description in any::<String>()) {
        let mut session = env.start_session();

        let head_before = session.tree().head().clone();
        let branches_before = session.tree().branch_tip_ids().clone();
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = session.versions_with_content();
        let versions_before = versions_with_content_before.iter().map(|(v, _)| v).collect::<HashSet<_>>();
        let version_contents_before = versions_with_content_before.iter().map(|(_, c)| c).collect::<HashSet<_>>();

        let target = session.tree().resolve_version_path(&target).id;
        session.update_description(target, new_description)?;

        let head_after = session.tree().head().clone();
        let branches_after = session.tree().branch_tip_ids().clone();
        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = session.versions_with_content();
        let versions_after = versions_with_content_after.iter().map(|(v, _)| v).collect::<HashSet<_>>();
        let version_contents_after = versions_with_content_after.iter().map(|(_, c)| c).collect::<HashSet<_>>();

        prop_assert_eq!(versioned_file_bytes_before, versioned_file_bytes_after, "reword does not modify versioned file");

        prop_assert_eq!(head_before, head_after, "reword does not modify head");

        prop_assert_eq!(branches_before, branches_after, "reword does not modify branches");

        let version_difference = HashSet::difference(&versions_before, &versions_after);
        prop_assert!(version_difference.count() <= 1, "reword modifies at most one version");

        prop_assert_eq!(version_contents_before, version_contents_after, "reword does not modify version contents");
    }
}
