use crate::error::Result;
use std::path::PathBuf;
use uuid::Uuid;

pub fn new_path() -> Result<PathBuf> {
    let runtime_dir = dirs::runtime()?;
    let file_name = Uuid::new_v4().to_string();

    Ok(runtime_dir.join(file_name))
}
