use derive_more::IsVariant;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, IsVariant)]
pub enum ContentBlobKind {
    Full,
    Patch,
}
