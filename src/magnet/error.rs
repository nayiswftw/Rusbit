use std::fmt;

#[derive(Debug)]
pub enum MagnetError {
    UnexpectedEnd,
    InvalidFormat(String),
    Utf8Error(std::str::Utf8Error),
}

impl fmt::Display for MagnetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MagnetError::UnexpectedEnd => write!(f, "Unexpected end of input"),
            MagnetError::InvalidFormat(s) => write!(f, "Invalid format: {}", s),
            MagnetError::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
        }
    }
}

impl std::error::Error for MagnetError {}

impl From<std::str::Utf8Error> for MagnetError {
    fn from(err: std::str::Utf8Error) -> Self {
        MagnetError::Utf8Error(err)
    }
}