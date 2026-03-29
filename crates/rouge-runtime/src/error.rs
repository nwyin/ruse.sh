/// Errors that can occur while running a `Program`.
#[derive(Debug, thiserror::Error)]
pub enum ProgramError {
    #[error("interrupted")]
    Interrupted,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("program panicked: {0}")]
    Panic(String),
    #[error("program killed")]
    Killed,
}
