use serde::{Deserialize, Serialize};
use std::fs;
use thiserror::Error;

static DEFAULT_CONFIG_STR: &str = include_str!("default_config.toml");

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalConfiguration {
    pub default_file_type_rules: DefaultFileTypeRules,
    pub file_type_rules: Vec<FileTypeRules>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DefaultFileTypeRules {
    pub create_patch_command: Vec<String>,
    pub apply_patch_command: Vec<String>,
    pub preview_command: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileTypeRules {
    pub extensions: Vec<String>,
    pub create_patch_command: Option<Vec<String>>,
    pub apply_patch_command: Option<Vec<String>>,
    pub preview_command: Option<Vec<String>>,
}

pub fn read_global() -> Result<GlobalConfiguration, Error> {
    let config_dir = biver_directories::config()?;
    fs::create_dir_all(&config_dir)?;
    let config_file = config_dir.join(CONFIG_FILE_NAME);

    if !fs::exists(&config_file)? {
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
    GetProjectDirs(#[from] biver_directories::GetProjectDirsError),
}

#[test]
fn default_config_is_valid() -> Result<(), toml::de::Error> {
    toml::from_str::<GlobalConfiguration>(DEFAULT_CONFIG_STR)?;
    Ok(())
}
