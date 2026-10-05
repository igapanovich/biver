use std::error::Error;
use std::path::PathBuf;
use std::{env, fs};
use uuid::Uuid;

pub fn create() -> Result<PathBuf, Box<dyn Error>> {
    let id = Uuid::new_v4();
    let temp_dir = env::temp_dir();
    let test_dir = temp_dir.join(format!("biver-tests-{}", id));
    fs::create_dir(&test_dir)?;
    Ok(test_dir)
}
