use std::collections::HashMap;

use zbus::interface;

use super::{
    DialogCommand, DialogOutcome, PolkitError,
    identity::{DbusIdentity, candidate_users},
    queue::AuthQueue,
    user_directory::SystemUsers,
};
use crate::request::DialogRequest;

/// The `org.freedesktop.PolicyKit1.AuthenticationAgent` object polkitd calls.
pub struct AuthenticationAgent {
    dialog: DialogCommand,
    queue: AuthQueue,
}

impl AuthenticationAgent {
    pub fn new(dialog: DialogCommand) -> Self {
        Self {
            dialog,
            queue: AuthQueue::default(),
        }
    }
}

#[interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl AuthenticationAgent {
    /// Replies once the user authenticated, or with `Cancelled` when they did not.
    async fn begin_authentication(
        &self,
        action_id: String,
        message: String,
        icon_name: String,
        _details: HashMap<String, String>,
        cookie: String,
        identities: Vec<DbusIdentity>,
    ) -> Result<(), PolkitError> {
        let users = candidate_users(&identities, &SystemUsers);
        if users.is_empty() {
            return Err(PolkitError::Failed(format!(
                "no local user may authenticate for {action_id}"
            )));
        }
        let mut ticket = self.queue.enqueue(&cookie);
        let Some(_turn) = ticket.wait_turn().await else {
            return Err(cancelled());
        };
        let request = DialogRequest {
            message,
            icon_name,
            cookie,
            users,
        };
        match self.dialog.authenticate(&request, ticket.cancelled()).await {
            Ok(DialogOutcome::Authenticated) => Ok(()),
            Ok(DialogOutcome::Dismissed) => Err(cancelled()),
            Err(error) => Err(PolkitError::Failed(error.to_string())),
        }
    }

    fn cancel_authentication(&self, cookie: &str) {
        self.queue.cancel(cookie);
    }
}

fn cancelled() -> PolkitError {
    PolkitError::Cancelled("the authentication was dismissed".to_owned())
}
