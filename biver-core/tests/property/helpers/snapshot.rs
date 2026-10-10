use biver_core::data::{BranchName, Head, Version, VersionId};
use biver_core::repository::Session;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Snapshot {
    pub versioned_file_content: Vec<u8>,
    pub has_uncommitted_changes: bool,
    pub head: Head,
    pub branches: HashMap<BranchName, VersionId>,
    pub versions: HashSet<VersionSnapshot>,
}

impl Snapshot {
    pub fn new(versioned_file_path: &Path, session: &Session) -> Self {
        Snapshot {
            versioned_file_content: fs::read(&versioned_file_path).unwrap(),
            has_uncommitted_changes: session.has_uncommitted_changes().unwrap(),
            head: session.tree().head().clone(),
            branches: session.tree().branch_tip_ids().clone(),
            versions: session
                .tree()
                .versions()
                .map(|v| VersionSnapshot::new(v.clone(), session))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct VersionSnapshot {
    pub version: Version,
    pub content_blob: Vec<u8>,
}

impl VersionSnapshot {
    pub fn new(version: Version, session: &Session) -> Self {
        let mut content_blob = Vec::new();
        session
            .raw_content_blob(version.id)
            .unwrap()
            .unwrap()
            .read_to_end(&mut content_blob)
            .unwrap();

        VersionSnapshot {
            version,
            content_blob,
        }
    }
}
