use serde::{Deserialize, Serialize};
use variantly::Variantly;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Variantly)]
pub enum ContentBlobKind {
    Full,
    Patch,
}
