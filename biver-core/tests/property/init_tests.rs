use crate::helpers::arb;
use crate::helpers::test_env::TestEnv;
use biver_core::repository;
use proptest::prelude::*;
use std::fs;

proptest! {
    #[test]
    fn init_main_group(init_bytes in arb::bytes()) {
        let env = TestEnv::new();

        fs::write(&env.versioned_file_path(), init_bytes)?;

        let init_ok = repository::initialize(env.config().clone(), env.versioned_file_path().to_path_buf(), None, None).unwrap();

        let tree = init_ok.session.tree();

        prop_assert_eq!(tree.branch_names().count(), 1, "init creates one branch");
        prop_assert_eq!(tree.versions().len(), 1, "init creates one version");
        prop_assert!(tree.head().is_branch(), "init sets head pointing at the branch");
    }
}
