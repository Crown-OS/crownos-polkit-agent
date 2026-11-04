use std::{env, process};

use nix::unistd::getuid;
use thiserror::Error;
use zbus::{Connection, proxy::CacheProperties};

use super::logind::{LoginManagerProxy, LoginSessionProxy, LoginUserProxy};

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("asking logind for the session failed: {0}")]
    Logind(#[from] zbus::Error),
    #[error("the user has no graphical session")]
    NoGraphicalSession,
}

/// The logind session this agent speaks for: `XDG_SESSION_ID` when started inside the session,
/// else the session of this process, else the user's graphical session (a systemd user service
/// belongs to no session itself).
pub async fn current_session_id(system: &Connection) -> Result<String, SessionError> {
    if let Some(id) = env::var("XDG_SESSION_ID").ok().filter(|id| !id.is_empty()) {
        return Ok(id);
    }
    let manager = LoginManagerProxy::new(system).await?;
    if let Ok(path) = manager.get_session_by_pid(process::id()).await {
        let session = LoginSessionProxy::builder(system)
            .path(path)?
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        return Ok(session.id().await?);
    }
    let user = LoginUserProxy::builder(system)
        .path(manager.get_user(getuid().as_raw()).await?)?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    let (id, _) = user.display().await?;
    if id.is_empty() {
        return Err(SessionError::NoGraphicalSession);
    }
    Ok(id)
}
