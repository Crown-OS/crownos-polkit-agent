use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("talking to the polkit helper failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("the polkit helper sent an unexpected line: {0:?}")]
    UnexpectedLine(String),
    #[error("no polkit helper socket or polkit-agent-helper-1 is installed")]
    HelperMissing,
    #[error("the polkit helper failed before asking for a password")]
    FailedBeforePrompt,
    #[error("an answer to the polkit helper must be a single line")]
    MultilineAnswer,
}
