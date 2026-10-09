use biver_core::repository::{Session, TryStartSessionResult};

pub trait UnwrapTryStartSessionExt {
    fn unwrap(self) -> Session;
}

impl UnwrapTryStartSessionExt for TryStartSessionResult {
    fn unwrap(self) -> Session {
        match self {
            TryStartSessionResult::Ok(session) => session,
            TryStartSessionResult::Uninitialized => panic!("not initialized"),
        }
    }
}
