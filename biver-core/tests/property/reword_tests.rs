use crate::helpers::arb;
use crate::helpers::version_path::ResolveVersionPathExt;
use crate::{DEFAULT_CASE_COUNT, GROUP_CASE_MULTIPLIER};
use biver_core::operations;
use proptest::prelude::*;
use std::collections::HashSet;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(DEFAULT_CASE_COUNT * GROUP_CASE_MULTIPLIER))]

    #[test]
    fn reword_main_group(env in arb::env(), target in arb::version_path(), new_description in any::<String>()) {
        let head_before = env.repository().head;
        let branches_before = env.repository().branches;
        let versioned_file_bytes_before = env.versioned_file_content();
        let versions_with_content_before = env.versions_with_content();
        let versions_before = versions_with_content_before.iter().map(|(v, _)| v).collect::<HashSet<_>>();
        let version_contents_before = versions_with_content_before.iter().map(|(_, c)| c).collect::<HashSet<_>>();

        let target = env.repository().resolve_version_path(&target).id.bs58();
        operations::reword(env.paths(), &mut env.repository(), &target, new_description).unwrap();

        let head_after = env.repository().head;
        let branches_after = env.repository().branches;
        let versioned_file_bytes_after = env.versioned_file_content();
        let versions_with_content_after = env.versions_with_content();
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
