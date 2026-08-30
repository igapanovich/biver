use crate::data::ContentBlobKind;
use crate::data::VersionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
    pub id: VersionId,
    pub creation_time: DateTime<Utc>,
    pub nickname: String,
    pub versioned_file_length: u64,
    pub versioned_file_xxh3_128: u128,
    pub description: String,
    pub parent: Option<VersionId>,
    pub content_blob_file_name: String,
    pub content_blob_kind: ContentBlobKind,
    pub preview_blob_file_name: Option<String>,
}

impl Version {
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}
