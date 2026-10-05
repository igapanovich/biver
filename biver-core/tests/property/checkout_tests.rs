use crate::helpers::arb;
use crate::helpers::repository_action::FileOperation;
use crate::helpers::repository_action::PositionInFile::FromStart;
use crate::helpers::version_path::RepositoryExt;
use biver_core::operations;
use proptest::prelude::*;

proptest! {
    #[test]
    fn checkout_succeeds_when_no_uncommitted_changes(env in arb::test_env(), checkout_target in arb::version_path()) {
        operations::discard(env.config(), env.paths(), &mut env.read_repository())?;

        let checkout_target_id = env.read_repository().resolve_version_path(&checkout_target).id;

        let outcome = operations::check_out(&env.config(), &env.paths(), &mut env.read_repository(), &checkout_target_id.bs58())?;

        assert!(outcome.is_ok())
    }

    #[test]
    fn checkout_fails_when_uncommitted_changes(env in arb::test_env(), checkout_target in arb::version_path(), bytes in arb::non_empty_bytes()) {
        env.run_versioned_file_operation(FileOperation::Insert { position: FromStart(0), bytes });

        let checkout_target_id = env.read_repository().resolve_version_path(&checkout_target).id;

        let outcome = operations::check_out(&env.config(), &env.paths(), &mut env.read_repository(), &checkout_target_id.bs58())?;

        assert!(outcome.is_has_uncommitted_changes())
    }
}
