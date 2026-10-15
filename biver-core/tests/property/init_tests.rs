use crate::helpers::extensions::UnwrapExt;
use crate::helpers::test_env::TestEnv;
use crate::helpers::{arb, assert_version_content_integrity};
use biver_core::repository;
use proptest::option;
use proptest::prelude::*;
use std::fs;
use std::io::Read;

proptest! {
    #[test]
    fn init_properties(init_bytes in arb::bytes(), branch_name in option::of(arb::branch_name()), description in any::<String>()) {
        let env = TestEnv::uninitialized();

        fs::write(&env.versioned_file_path(), &init_bytes)?;

        let init_result = repository::initialize(env.config().clone(), env.versioned_file_path(), branch_name, description)?;

        prop_assert!(init_result.is_ok());

        let session = repository::try_start_session(env.versioned_file_path())?.unwrap();
        let tree = session.tree();

        let mut root_content = Vec::new();
        session
            .version_full_content(tree.root_version().id)?
            .unwrap()
            .read_to_end(&mut root_content)?;

        prop_assert_eq!(tree.branch_names().count(), 1, "init creates one branch");
        prop_assert_eq!(tree.versions().count(), 1, "init creates one version");
        prop_assert!(tree.head().is_branch(), "init sets head pointing at the branch");
        prop_assert_eq!(root_content, init_bytes.as_ref(), "root version has the same content as versioned file");
        prop_assert!(!session.has_uncommitted_changes()?, "init doesn't leave uncommitted changes");

        assert_version_content_integrity(session)?;
    }
}
