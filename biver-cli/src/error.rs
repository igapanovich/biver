use std::fmt::Debug;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Core(#[from] biver_core::Error),

    #[error("configuration error: {0}")]
    Configuration(#[from] biver_configuration::Error),

    #[error("failed to get project directories")]
    GetProjectDirs(#[from] biver_directories::GetProjectDirsError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("eframe/egui error: {0}")]
    Eframe(#[from] eframe::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("toml deserialization error: {0}")]
    TomlDeserialization(#[from] toml::de::Error),

    #[error("failed to initialize tracing appender: {0}")]
    TracingAppender(#[from] tracing_appender::rolling::InitError),

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn from_string(str: impl Into<String>) -> Self {
        Self::Other(str.into())
    }
}
