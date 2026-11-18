//! Typed engine errors. IPC still surfaces `String`; convert with [`EngineError::into_message`].

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("{0}")]
    Message(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path error: {0}")]
    Path(String),
    #[error("Validation: {0}")]
    Validation(String),
    #[error("Session: {0}")]
    Session(String),
    #[error("Safety: {0}")]
    Safety(String),
}

impl EngineError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }

    pub fn into_message(self) -> String {
        self.to_string()
    }
}

impl From<String> for EngineError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for EngineError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

pub type EngineResult<T> = Result<T, EngineError>;
