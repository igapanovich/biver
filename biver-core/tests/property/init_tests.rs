use crate::helpers::arb;
use crate::helpers::extensions::UnwrapExt;
use crate::helpers::test_env::TestEnv;
use biver_core::repository;
use proptest::prelude::*;
use std::fs;
use std::io::Read;

proptest! {
    #[test]
    fn init_properties(init_bytes in arb::bytes(), branch_name in arb::branch_name_option(), description in any::<String>()) {
        let env = TestEnv::uninitialized();

        fs::write(&env.versioned_file_path(), &init_bytes)?;

        let init_result = repository::initialize(env.config().clone(), env.versioned_file_path().to_path_buf(), branch_name, description)?;

        prop_assert!(init_result.is_ok());

        let session = repository::try_start_session(env.config().clone(), env.versioned_file_path().to_path_buf())?.unwrap();
        let tree = session.tree();

        let mut root_content = Vec::new();
        session
            .raw_content_blob(tree.root_version().id)?
            .unwrap()
            .read_to_end(&mut root_content)?;

        prop_assert_eq!(tree.branch_names().count(), 1, "init creates one branch");
        prop_assert_eq!(tree.versions().count(), 1, "init creates one version");
        prop_assert!(tree.head().is_branch(), "init sets head pointing at the branch");
        prop_assert_eq!(root_content, init_bytes.as_ref(), "root version has the same content as versioned file");
    }
}
