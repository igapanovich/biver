use biver_core::data::{BranchName, Head, Version, VersionId};
use biver_core::repository::Session;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Snapshot {
    pub versioned_file_content: Vec<u8>,
    pub has_uncommitted_changes: bool,
    pub head: Head,
    pub branches: HashMap<BranchName, VersionId>,
    pub versions: HashSet<Version>,
}

impl Snapshot {
    pub fn new(versioned_file_path: &Path, session: &Session) -> Self {
        Snapshot {
            versioned_file_content: fs::read(&versioned_file_path).unwrap(),
            has_uncommitted_changes: session.has_uncommitted_changes().unwrap(),
            head: session.tree().head().clone(),
            branches: session.tree().branch_tip_map().clone(),
            versions: session.tree().versions().cloned().collect(),
        }
    }
}
