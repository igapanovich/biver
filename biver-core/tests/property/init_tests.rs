use crate::helpers::arb;
use crate::helpers::test_env::TestEnv;
use biver_core::repository;
use proptest::prelude::*;
use std::fs;
use std::io::Read;

proptest! {
    #[test]
    fn init_main_group(init_bytes in arb::bytes()) {
        let env = TestEnv::uninitialized();

        fs::write(&env.versioned_file_path(), &init_bytes)?;

        let init_ok = repository::initialize(env.config().clone(), env.versioned_file_path().to_path_buf(), None, None).unwrap();

        let session = init_ok.session;
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
