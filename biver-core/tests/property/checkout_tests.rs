use crate::helpers::arb;
use crate::helpers::repository_action::FileOperation;
use crate::helpers::repository_action::PositionInFile::FromStart;
use crate::helpers::version_path::RepositoryExt;
use biver_core::operations;
use proptest::prelude::*;

proptest! {
    #[test]
    fn checkout_succeeds_when_no_uncommitted_changes(env in arb::test_env(), checkout_path in arb::version_path()) {
        operations::discard(env.config(), env.paths(), &mut env.read_repository())?;

        let checkout_target = env.read_repository().resolve_version_path(&checkout_path);

        let outcome = operations::check_out(&env.config(), &env.paths(), &mut env.read_repository(), &checkout_target)?;

        prop_assert!(outcome.is_ok())
    }

    #[test]
    fn checkout_fails_when_there_are_uncommitted_changes(env in arb::test_env(), checkout_path in arb::version_path(), bytes in arb::non_empty_bytes()) {
        env.run_versioned_file_operation(FileOperation::Insert { position: FromStart(0), bytes });

        let checkout_target = env.read_repository().resolve_version_path(&checkout_path);

        let outcome = operations::check_out(&env.config(), &env.paths(), &mut env.read_repository(), &checkout_target)?;

        prop_assert!(outcome.is_has_uncommitted_changes())
    }

    #[test]
    fn checkout_does_not_modify_versions(env in arb::test_env(), checkout_path in arb::version_path()) {
        let versions_with_content_before = env.read_versions_with_content();

        let checkout_target = env.read_repository().resolve_version_path(&checkout_path);

        operations::check_out(&env.config(), &env.paths(), &mut env.read_repository(), &checkout_target)?;

        let versions_with_content_after = env.read_versions_with_content();

        prop_assert_eq!(versions_with_content_before, versions_with_content_after);
    }
}
