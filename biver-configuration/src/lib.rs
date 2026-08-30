use serde::{Deserialize, Serialize};
use std::fs;
use thiserror::Error;

static DEFAULT_CONFIG_STR: &str = include_str!("default_config.toml");

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Configuration {
    pub create_patch_command: Vec<String>,
    pub apply_patch_command: Vec<String>,
    pub file_type_rules: Vec<FileTypeRule>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTypeRule {
    pub extensions: Vec<String>,
    pub preview_command: Option<Vec<String>>,
}

pub fn read() -> Result<Configuration, Error> {
    let config_dir = dirs::config()?;
    fs::create_dir_all(&config_dir)?;
    let config_file = config_dir.join(CONFIG_FILE_NAME);

    if !config_file.exists() {
        fs::write(&config_file, DEFAULT_CONFIG_STR)?;
    }

    let config_str = fs::read_to_string(config_file)?;
    let config = toml::de::from_str(&config_str)?;

    Ok(config)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("deserialization error: {0}")]
    Deserialization(#[from] toml::de::Error),

    #[error("{0}")]
    GetProjectDirs(#[from] dirs::GetProjectDirsError),
}

#[test]
fn default_config_is_valid() -> Result<(), toml::de::Error> {
    toml::from_str::<Configuration>(DEFAULT_CONFIG_STR)?;
    Ok(())
}
