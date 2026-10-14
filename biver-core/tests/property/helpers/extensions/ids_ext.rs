use crate::helpers::snapshot::VersionSnapshot;
use biver_core::data::{Version, VersionId};
use std::collections::HashSet;

pub trait IdsExt<'a, T> {
    fn ids(&'a mut self) -> HashSet<VersionId>;
}

impl<'a, T> IdsExt<'a, Version> for T
where
    T: Iterator<Item = &'a Version>,
{
    fn ids(&'a mut self) -> HashSet<VersionId> {
        self.map(|v| v.id).collect()
    }
}

impl<'a, T> IdsExt<'a, VersionSnapshot> for T
where
    T: Iterator<Item = &'a VersionSnapshot>,
{
    fn ids(&'a mut self) -> HashSet<VersionId> {
        self.map(|v| v.version.id).collect()
    }
}
