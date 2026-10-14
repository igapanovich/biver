use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    GetProjectDirs(#[from] biver_directories::GetProjectDirsError),

    #[error("cbor serialization error: {0}")]
    CborSerialization(#[from] ciborium::ser::Error<std::io::Error>),

    #[error("cbor deserialization error: {0}")]
    CborDeserialization(#[from] ciborium::de::Error<std::io::Error>),

    #[error("toml serialization error: {0}")]
    TomlSerialization(#[from] toml::ser::Error),

    #[error("toml deserialization error: {0}")]
    TomlDeserialization(#[from] toml::de::Error),

    #[error("external command error: {0}")]
    ExternalCommand(String),

    #[error("error converting UTF string: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("attempted to use a broken instance")]
    Broken,

    #[error("repository is locked")]
    RepositoryLocked,

    #[error("repository schema is incompatible: {0}")]
    IncompatibleSchemaVersion(String),
}

pub type Result<T> = std::result::Result<T, Error>;
