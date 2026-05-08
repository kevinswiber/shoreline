pub type Result<T> = std::result::Result<T, ShoreError>;

#[derive(Debug, thiserror::Error)]
pub enum ShoreError {
    #[error("{0}")]
    Message(String),
}
