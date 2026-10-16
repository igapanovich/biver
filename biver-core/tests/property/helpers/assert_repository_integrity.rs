use crate::helpers::extensions::IteratorExt;
use biver_core::repository::{CheckOutVersionResult, Paths, Session};
use itertools::Itertools;
use proptest::prelude::TestCaseError;
use proptest::{prop_assert, prop_assert_eq};
use std::fs;

pub fn assert_repository_integrity(mut session: Session) -> Result<(), TestCaseError> {
    let paths = Paths::new(session.versioned_file_path());

    assert_full_content_dir_integrity(&session, &paths)?;
    assert_version_content_integrity(&mut session)?;

    Ok(())
}

fn assert_full_content_dir_integrity(
    session: &Session,
    paths: &Paths,
) -> Result<(), TestCaseError> {
    let branch_tip_full_contents = session
        .tree()
        .branch_tip_ids()
        .values()
        .map(|id| paths.version_full_content_file(*id))
        .collect_set();

    let all_full_contents = fs::read_dir(paths.full_content_dir())?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|rd| rd.path())
        .collect_set();

    prop_assert_eq!(branch_tip_full_contents, all_full_contents);

    Ok(())
}

fn assert_version_content_integrity(session: &mut Session) -> Result<(), TestCaseError> {
    session.discard()?;

    let version_ids = session.tree().versions().map(|v| v.id).collect_vec();

    for version_id in version_ids {
        let check_out_result = session.check_out_version(version_id)?;
        prop_assert_eq!(
            check_out_result,
            CheckOutVersionResult::Ok,
            "version content integrity"
        );
        prop_assert!(
            !session.has_uncommitted_changes()?,
            "version content integrity"
        );
    }

    Ok(())
}
