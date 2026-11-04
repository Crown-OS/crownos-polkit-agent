//! The CrownOS polkit authentication agent: a resident D-Bus daemon without any UI, and the
//! CrownUI password prompt it starts for each authentication request.

pub mod agent;
pub mod auth;
#[cfg(feature = "dialog")]
pub mod dialog;
pub mod request;
mod util;
