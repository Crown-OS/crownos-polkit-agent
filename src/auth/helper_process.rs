use std::{path::Path, process::Stdio};

use tokio::process::{Child, Command};

use super::AuthError;

const HELPER_PATHS: [&str; 2] = [
    "/usr/lib/polkit-1/polkit-agent-helper-1",
    "/usr/libexec/polkit-agent-helper-1",
];

/// Spawns the setuid helper for `user`; it is killed when the returned child drops.
pub fn spawn(user: &str) -> Result<Child, AuthError> {
    let program = HELPER_PATHS
        .into_iter()
        .find(|path| Path::new(path).exists())
        .ok_or(AuthError::HelperMissing)?;
    Ok(Command::new(program)
        .arg(user)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?)
}
