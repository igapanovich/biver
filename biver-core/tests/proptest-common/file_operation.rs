use crate::helpers::byte_chunk::ByteChunk;
use crate::helpers::extensions::IndexExt;
use proptest::sample::Index;
use std::fmt::Debug;
use std::fs;
use std::path::Path;

#[derive(Clone)]
pub enum FileOperation {
    Overwrite(ByteChunk),
    Splice {
        range_start: Index,
        range_length: Option<Index>,
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
                "Splice({}, {:?}){:?}",
                range_start.debug_fraction(),
                range_length.map(|r| r.debug_fraction()),
                bytes
            ),
        }
    }
}

impl FileOperation {
    pub fn execute(&self, path: &Path) {
        match self {
            FileOperation::Overwrite(bytes) => {
                fs::write(&path, &bytes).unwrap();
            }
            FileOperation::Splice {
                range_start,
                range_length,
                bytes,
            } => {
                if !fs::exists(path).unwrap() {
                    fs::write(&path, bytes).unwrap();
                    return;
                }

                let mut content = fs::read(&path).unwrap();
                if content.is_empty() {
                    fs::write(&path, &bytes).unwrap();
                    return;
                }

                let range_start = range_start.index(content.len());

                let range_length = if let Some(range_length) = range_length {
                    range_length.index(content.len() - range_start)
                } else {
                    0
                };

                let range_end = range_start + range_length;
                content.splice(range_start..range_end, bytes.as_ref().iter().copied());
                fs::write(&path, &content).unwrap();
            }
        }
    }
}

pub trait FileOperations {
    fn execute(self, path: &Path);
}

impl<'a, T: IntoIterator<Item = &'a FileOperation>> FileOperations for T {
    fn execute(self, path: &Path) {
        for f in self {
            f.execute(path);
        }
    }
}
