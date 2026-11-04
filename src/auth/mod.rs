//! The conversation with `polkit-agent-helper-1`, which runs PAM as root and tells polkitd when
//! the user proved who they are.

pub mod conversation;
pub mod error;
pub mod helper;
mod helper_process;
mod helper_socket;
pub mod message;

pub use conversation::{Conversation, HelperLink};
pub use error::AuthError;
pub use helper::Helper;
pub use message::{Echo, HelperMessage};
