use std::fmt::Debug;
use std::vec;

#[derive(Clone)]
pub struct ByteChunk(Vec<u8>);

impl From<Vec<u8>> for ByteChunk {
    fn from(bytes: Vec<u8>) -> ByteChunk {
        ByteChunk(bytes)
    }
}

impl From<ByteChunk> for Vec<u8> {
    fn from(chunk: ByteChunk) -> Vec<u8> {
        chunk.0
    }
}

impl AsRef<[u8]> for ByteChunk {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl IntoIterator for ByteChunk {
    type Item = u8;
    type IntoIter = vec::IntoIter<u8>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Debug for ByteChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;

        let len = self.0.len();
        let print_len = len.min(12);

        for i in 0..print_len {
            write!(f, "{:02x}", self.0[i])?;

            if i < print_len - 1 {
                write!(f, ", ")?;
            }
        }

        if len > print_len {
            write!(f, ", ...{}", len - print_len)?;
        }

        write!(f, "]")?;

        Ok(())
    }
}
