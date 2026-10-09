#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("nobody has joined yet")]
    NoParticipants,
    #[error("nothing has been picked yet")]
    NothingPicked,
    #[error("storage error: {0}")]
    Storage(String),
}

pub type Result<T> = std::result::Result<T, Error>;
