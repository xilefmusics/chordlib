use std::error;
use std::fmt;
use std::io;

#[derive(Debug, PartialEq)]
pub enum Error {
    Parse(String),
    Filesystem(String),
    Serialize(String),
    Http(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "ParseError ({})", message),
            Self::Filesystem(message) => write!(f, "FilesystemError ({})", message),
            Self::Serialize(message) => write!(f, "Serialize ({})", message),
            Self::Http(message) => write!(f, "Http ({})", message),
            Self::Other(message) => write!(f, "{}", message),
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Filesystem(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialize(err.to_string())
    }
}

#[cfg(feature = "download")]
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err.to_string())
    }
}
