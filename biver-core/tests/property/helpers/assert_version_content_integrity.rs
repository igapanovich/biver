use biver_core::repository::{CheckOutVersionResult, Session};
use itertools::Itertools;
use proptest::prelude::TestCaseError;
use proptest::{prop_assert, prop_assert_eq};

pub fn assert_version_content_integrity(mut session: Session) -> Result<(), TestCaseError> {
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
