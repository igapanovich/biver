use std::env;
use std::path::PathBuf;
use uuid::Uuid;

pub fn new_path() -> PathBuf {
    let temp_dir = env::temp_dir();
    let file_name = Uuid::new_v4().to_string();

    temp_dir.join(file_name)
}
