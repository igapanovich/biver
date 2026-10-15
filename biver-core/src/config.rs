use serde::{Deserialize, Serialize};

pub type ExternalCommand = Vec<String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub create_patch_command: ExternalCommand,
    pub apply_patch_command: ExternalCommand,
    pub preview_command: Option<ExternalCommand>,
}
