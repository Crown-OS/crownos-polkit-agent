use std::{
    env,
    future::Future,
    io::{self, ErrorKind},
    path::PathBuf,
    process::Stdio,
};

use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::request::{DialogRequest, RequestError};

#[derive(Debug, Error)]
pub enum DialogError {
    #[error("starting the authentication dialog failed: {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Request(#[from] RequestError),
}

/// How a dialog process ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogOutcome {
    /// The helper reported `SUCCESS`, so polkitd already knows.
    Authenticated,
    /// The user cancelled, the dialog failed, or polkitd cancelled it.
    Dismissed,
}

/// The program that shows one authentication and exits 0 once the user authenticated.
#[derive(Debug, Clone)]
pub struct DialogCommand {
    program: PathBuf,
    args: Vec<String>,
}

impl DialogCommand {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    /// `name` in the directory of the running executable.
    pub fn beside_current_exe(name: &str) -> io::Result<Self> {
        Ok(Self::new(env::current_exe()?.with_file_name(name)))
    }

    #[must_use]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Runs the dialog for `request` until it exits, or kills it once `cancelled` resolves.
    pub async fn authenticate(
        &self,
        request: &DialogRequest,
        cancelled: impl Future<Output = ()>,
    ) -> Result<DialogOutcome, DialogError> {
        let encoded = request.encode()?;
        let mut dialog = Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        if let Some(mut stdin) = dialog.stdin.take() {
            match stdin.write_all(encoded.as_bytes()).await {
                Err(error) if error.kind() != ErrorKind::BrokenPipe => return Err(error.into()),
                _ => {}
            }
        }
        tokio::select! {
            status = dialog.wait() => Ok(if status?.success() {
                DialogOutcome::Authenticated
            } else {
                DialogOutcome::Dismissed
            }),
            () = cancelled => {
                dialog.kill().await?;
                Ok(DialogOutcome::Dismissed)
            }
        }
    }
}
