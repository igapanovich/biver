use crate::data::{BranchName, VersionId};
use crate::error::Result;
use crate::repository::RepositoryPaths;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const BRANCH_TIP_CACHE_CAPACITY: usize = 5;

#[derive(Debug, Serialize, Deserialize)]
struct CacheData {
    branch_tips: VecDeque<(BranchName, VersionId)>,
}

impl Default for CacheData {
    fn default() -> CacheData {
        CacheData {
            branch_tips: VecDeque::new(),
        }
    }
}

#[derive(Debug)]
pub struct Cache {
    paths: RepositoryPaths,
    removed_ids: HashSet<VersionId>,
    data: CacheData,
}

impl Cache {
    pub fn read(paths: &RepositoryPaths) -> Result<Cache> {
        let data = if fs::exists(paths.cache_data_file())? {
            let data_bytes = fs::read(paths.cache_data_file())?;
            serde_json::from_slice(&data_bytes)?
        } else {
            CacheData::default()
        };

        Ok(Cache {
            paths: paths.clone(),
            data,
            removed_ids: HashSet::new(),
        })
    }

    pub fn get(&self, version_id: VersionId) -> Option<PathBuf> {
        if self.all_cached_ids().contains(&version_id) {
            Some(self.paths.version_content_cache_file(version_id))
        } else {
            None
        }
    }

    pub fn set_branch_tip(
        &mut self,
        branch_name: BranchName,
        version_id: VersionId,
        content: &Path,
    ) -> Result<()> {
        self.remove_branch_tip(&branch_name);
        self.add_branch_tip(branch_name, version_id);

        let path = self.paths.version_content_cache_file(version_id);
        fs::copy(&content, &path)?;

        Ok(())
    }

    pub fn invalidate_branch_tip(&mut self, branch_name: &BranchName) {
        self.remove_branch_tip(branch_name);
    }

    pub fn invalidate_all(&mut self) {
        let branch_names = self
            .data
            .branch_tips
            .iter()
            .map(|(b, _)| b)
            .cloned()
            .collect_vec();

        for name in branch_names {
            self.invalidate_branch_tip(&name);
        }
    }

    pub fn flush(&mut self) -> Result<()> {
        self.shrink_branch_tips();
        self.write_data()?;
        self.cleanup_removed_ids()?;

        Ok(())
    }

    fn remove_branch_tip(&mut self, branch_name: &BranchName) -> Option<VersionId> {
        let pos = self
            .data
            .branch_tips
            .iter()
            .position(|(b, _)| b == branch_name);

        if let Some(pos) = pos {
            let (_, old_id) = self.data.branch_tips.remove(pos).unwrap();
            self.removed_ids.insert(old_id);
            Some(old_id)
        } else {
            None
        }
    }

    fn add_branch_tip(&mut self, branch_name: BranchName, version_id: VersionId) {
        self.data.branch_tips.push_front((branch_name, version_id));
    }

    fn all_cached_ids(&self) -> HashSet<VersionId> {
        self.data.branch_tips.iter().map(|(_, id)| *id).collect()
    }

    fn shrink_branch_tips(&mut self) {
        let over_capacity = self
            .data
            .branch_tips
            .len()
            .saturating_sub(BRANCH_TIP_CACHE_CAPACITY);

        for _ in 0..over_capacity {
            self.data.branch_tips.pop_back();
        }
    }

    fn cleanup_removed_ids(&mut self) -> Result<()> {
        let alive_ids = self.all_cached_ids();
        let removed_ids = self
            .removed_ids
            .difference(&alive_ids)
            .copied()
            .collect_vec();
        self.removed_ids.clear();

        for id in removed_ids {
            let path = self.paths.version_content_cache_file(id);
            fs::remove_file(path)?;
        }

        Ok(())
    }

    fn write_data(&self) -> Result<()> {
        let cache_str = serde_json::to_string_pretty(&self.data)?;
        fs::create_dir_all(self.paths.cache_dir())?;
        fs::write(self.paths.cache_data_file(), &cache_str)?;
        Ok(())
    }
}
