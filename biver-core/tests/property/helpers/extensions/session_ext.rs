use biver_core::data::Version;
use biver_core::repository::Session;
use std::collections::HashSet;
use std::io::Read;

pub trait SessionExt {
    fn versions_with_content(&self) -> HashSet<(Version, Vec<u8>)>;
}

impl SessionExt for Session {
    fn versions_with_content(&self) -> HashSet<(Version, Vec<u8>)> {
        self.tree()
            .versions()
            .into_iter()
            .map(|v| {
                let mut bytes = Vec::new();
                self.raw_content_blob(v.id)
                    .unwrap()
                    .unwrap()
                    .read_to_end(&mut bytes)
                    .unwrap();
                (v.clone(), bytes)
            })
            .collect()
    }
}
