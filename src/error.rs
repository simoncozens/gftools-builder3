use std::error::Error;
use thiserror::Error;
use tokio::{io, task::JoinError};

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ApplicationError {
    #[error("build failed")]
    Build,
    #[error("default output not found")]
    DefaultOutputNotFound,
    #[error("{0}")]
    Other(String),
}

impl From<Box<dyn Error>> for ApplicationError {
    fn from(error: Box<dyn Error>) -> Self {
        Self::Other(error.to_string())
    }
}

impl From<io::Error> for ApplicationError {
    fn from(error: io::Error) -> Self {
        Self::Other(error.to_string())
    }
}

impl From<JoinError> for ApplicationError {
    fn from(error: JoinError) -> Self {
        Self::Other(error.to_string())
    }
}
