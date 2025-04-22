use std::fmt;

#[derive(Debug)]
pub enum Error {
    NoAvailableBackend,
    TranslationFailed(String),
    ConfigError(String),
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoAvailableBackend => write!(f, "No available translation backend"),
            Error::TranslationFailed(msg) => write!(f, "Translation failed: {}", msg),
            Error::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::SerdeError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::SerdeError(error)
    }
}
