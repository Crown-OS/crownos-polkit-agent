//! The resident half: registers with polkitd for this session and, per request, runs the dialog
//! process. It holds no UI, so it stays a few hundred kilobytes while idle.

pub mod daemon;
pub mod dialog_process;
pub mod error;
pub mod identity;
pub mod interface;
mod logind;
pub mod queue;
pub mod register;
pub mod session;
pub mod user_directory;

pub use dialog_process::{DialogCommand, DialogOutcome};
pub use error::PolkitError;
pub use interface::AuthenticationAgent;
pub use register::AGENT_PATH;
