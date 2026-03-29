use std::env::temp_dir;
use std::path::PathBuf;
use uuid::Uuid;

pub fn path() -> PathBuf {
    let file_name = Uuid::new_v4().to_string();
    temp_dir().join(file_name)
}
