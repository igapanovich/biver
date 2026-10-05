#![allow(dead_code)]

#[derive(Debug, Clone)]
pub enum RepositoryAction {
    ModifyVersionedFile(Vec<u8>),
    Init(Vec<u8>),
    Commit(Option<Vec<u8>>),
}
