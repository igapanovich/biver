use crate::error::{Error, Result, Severity};
use biver_core::repository::{Session, TryStartSessionResult};

pub trait ResultExt {
    type Value;

    fn result(self) -> Result<Self::Value>;
}

impl ResultExt for TryStartSessionResult {
    type Value = Session;

    fn result(self) -> Result<Session> {
        match self {
            TryStartSessionResult::Ok(session) => Ok(session),
            TryStartSessionResult::NotInitialized => Err(Error {
                message: "Not initialized".to_string(),
                severity: Severity::Error,
            }),
        }
    }
}
