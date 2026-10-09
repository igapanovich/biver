use crate::helpers::checkout_target::CheckoutTarget;
use proptest::sample::Index;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum RepositoryAction {
    ModifyVersionedFile(FileOperation),
    Init,
    Commit,
    Discard,
    CheckOut(CheckoutTarget),
}

#[derive(Debug, Clone)]
pub enum FileOperation {
    Overwrite(Vec<u8>),
    Splice {
        range_start: Index,
        range_length: Index,
        bytes: Vec<u8>,
    },
}
