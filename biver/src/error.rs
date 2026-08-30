use std::fmt::{Debug, Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
    pub severity: Severity,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let severity = match self.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
        };

        write!(f, "{}: {}", severity, self.message)
    }
}

impl From<eframe::Error> for Error {
    fn from(value: eframe::Error) -> Self {
        Self {
            message: format!("eframe/egui failure: {}", value),
            severity: Severity::Error,
        }
    }
}

impl From<image::ImageError> for Error {
    fn from(value: image::ImageError) -> Self {
        Self {
            message: format!("image failure: {}", value),
            severity: Severity::Error,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self {
            message: format!("io failure: {}", value),
            severity: Severity::Error,
        }
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Self {
            message: format!("deserialization failure: {}", value),
            severity: Severity::Error,
        }
    }
}

impl From<biver_core::error::Error> for Error {
    fn from(value: biver_core::error::Error) -> Self {
        Self {
            message: value.to_string(),
            severity: Severity::Error,
        }
    }
}

impl From<biver_configuration::Error> for Error {
    fn from(value: biver_configuration::Error) -> Self {
        Self {
            message: format!("configuration failure: {}", value),
            severity: Severity::Error,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Error,
    Warning,
}

pub fn error<T>(message: impl Into<String>) -> Result<T> {
    Err(Error {
        message: message.into(),
        severity: Severity::Error,
    }
    .into())
}

pub fn warning<T>(message: impl Into<String>) -> Result<T> {
    Err(Error {
        message: message.into(),
        severity: Severity::Warning,
    }
    .into())
}
