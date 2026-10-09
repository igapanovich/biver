use crate::helpers::byte_chunk::ByteChunk;
use crate::helpers::extensions::IndexExt;
use crate::helpers::version_path::VersionPath;
use proptest::sample::Index;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum RepositoryAction {
    ModifyVersionedFile(FileOperation),
    Init,
    Commit,
    Discard,
    CheckOutBranch(Index),
    CheckOutVersion(VersionPath),
}

#[derive(Clone)]
pub enum FileOperation {
    Overwrite(ByteChunk),
    Splice {
        range_start: Index,
        range_length: Index,
        bytes: ByteChunk,
    },
}

impl Debug for FileOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOperation::Overwrite(bytes) => write!(f, "Overwrite{:?}", bytes),
            FileOperation::Splice {
                range_start,
                range_length,
                bytes,
            } => write!(
                f,
                "Splice({}, {}){:?}",
                range_start.debug_fraction(),
                range_length.debug_fraction(),
                bytes
            ),
        }
    }
}
