use crate::error::{Error, Result, Severity};
use biver_core::repository::{Session, TryStartSessionResult};

pub trait OkExt {
    type Value;
    fn ok(self) -> Result<Self::Value>;
}

impl OkExt for TryStartSessionResult {
    type Value = Session;

    fn ok(self) -> Result<Session> {
        match self {
            TryStartSessionResult::Ok(session) => Ok(session),
            TryStartSessionResult::NotInitialized => Err(Error {
                message: "Not initialized".to_string(),
                severity: Severity::Error,
            }),
        }
    }
}
