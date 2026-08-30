use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    GetProjectDirs(#[from] dirs::GetProjectDirsError),

    #[error("json deserialization error: {0}")]
    JsonDeserialization(#[from] serde_json::Error),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("external command error: {0}")]
    ExternalCommand(String),

    #[error("error converting UTF string: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

pub type Result<T> = std::result::Result<T, Error>;
