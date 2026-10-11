use biver_core::repository::{Session, TryStartSessionResult};

pub trait UnwrapExt {
    type Value;

    fn unwrap(self) -> Self::Value;
}

impl UnwrapExt for TryStartSessionResult {
    type Value = Session;

    fn unwrap(self) -> Self::Value {
        match self {
            TryStartSessionResult::Ok(session) => session,
            TryStartSessionResult::NotInitialized => panic!("Result was NotInitialized"),
        }
    }
}
