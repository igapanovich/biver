use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum RepositoryAction {
    ModifyVersionedFile(FileOperation),
    Init,
    Commit,
    Discard,
}

#[derive(Debug, Clone)]
pub enum FileOperation {
    Overwrite(Vec<u8>),
    Insert {
        position: PositionInFile,
        bytes: Vec<u8>,
    },
    RemoveRange {
        start: PositionInFile,
        length: usize,
    },
}

#[derive(Debug, Clone)]
pub enum PositionInFile {
    FromStart(usize),
    FromEnd(usize),
}
