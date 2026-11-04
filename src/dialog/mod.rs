//! The per-request half: a full-screen CrownUI overlay that runs the helper conversation itself
//! and exits 0 once the user authenticated.

pub mod app;
pub mod authenticator;
mod autofocus;
pub mod root;
pub mod shake;
pub mod shell;
pub mod state;

pub use app::run;
pub use shell::{Intent, shell};
pub use state::{DialogState, Feedback};
