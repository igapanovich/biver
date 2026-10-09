use crate::helpers::arb;
use crate::helpers::test_env::TestEnv;
use biver_core::operations;
use proptest::prelude::*;
use std::fs;

proptest! {
    #[test]
    fn init_main_group(init_bytes in arb::bytes()) {
        let env = TestEnv::new();

        fs::write(&env.paths().versioned_file, init_bytes)?;

        operations::init(env.config(), env.paths(), None, None)?;

        let repository = env.repository();

        prop_assert_eq!(repository.branches.len(), 1, "init creates one branch");
        prop_assert_eq!(repository.versions.len(), 1, "init creates one version");
        prop_assert!(repository.head.is_branch(), "init sets head pointing at the branch");
    }
}
