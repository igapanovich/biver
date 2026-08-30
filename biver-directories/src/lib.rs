use directories::ProjectDirs;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub fn config() -> Result<PathBuf, GetProjectDirsError> {
    let project_dirs = project_dirs()?;
    Ok(project_dirs.config_local_dir().to_path_buf())
}
pub fn runtime() -> Result<PathBuf, GetProjectDirsError> {
    let project_dirs = project_dirs()?;
    Ok(project_dirs
        .runtime_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| project_dirs.data_local_dir().join("temp")))
}

fn project_dirs() -> Result<ProjectDirs, GetProjectDirsError> {
    ProjectDirs::from("", "", "biver").ok_or_else(|| GetProjectDirsError)
}

#[derive(Debug)]
pub struct GetProjectDirsError;

impl Error for GetProjectDirsError {}

impl Display for GetProjectDirsError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Could not determine project directories")
    }
}
