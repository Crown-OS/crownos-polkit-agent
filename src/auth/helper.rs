use tokio::{
    io::{AsyncRead, AsyncWrite, BufReader},
    process::Child,
};

use super::{AuthError, Conversation, HelperLink, HelperMessage, helper_process, helper_socket};

type HelperReader = BufReader<Box<dyn AsyncRead + Send + Unpin>>;
type HelperWriter = Box<dyn AsyncWrite + Send + Unpin>;

/// One PAM attempt by `polkit-agent-helper-1`; dropping it hangs up and ends the attempt.
pub struct Helper {
    conversation: Conversation<HelperReader, HelperWriter>,
    _process: Option<Child>,
}

impl Helper {
    /// Starts an attempt for `user` through the helper socket when polkit offers one, else by
    /// spawning the setuid helper, and hands it the cookie the way polkit's own agents do.
    pub async fn start(user: &str, cookie: &str) -> Result<Self, AuthError> {
        let mut helper = match helper_socket::connect().await {
            Some(stream) => {
                let (reader, writer) = stream.into_split();
                let mut helper = Self::new(Box::new(reader), Box::new(writer), None);
                helper.conversation.send_line(user).await?;
                helper
            }
            None => {
                let mut process = helper_process::spawn(user)?;
                let (Some(stdout), Some(stdin)) = (process.stdout.take(), process.stdin.take())
                else {
                    return Err(AuthError::HelperMissing);
                };
                Self::new(Box::new(stdout), Box::new(stdin), Some(process))
            }
        };
        helper.conversation.send_line(cookie).await?;
        Ok(helper)
    }

    fn new(
        reader: Box<dyn AsyncRead + Send + Unpin>,
        writer: HelperWriter,
        process: Option<Child>,
    ) -> Self {
        Self {
            conversation: Conversation::new(BufReader::new(reader), writer),
            _process: process,
        }
    }
}

impl HelperLink for Helper {
    async fn next_message(&mut self) -> Result<HelperMessage, AuthError> {
        self.conversation.next_message().await
    }

    async fn send_line(&mut self, line: &str) -> Result<(), AuthError> {
        self.conversation.send_line(line).await
    }
}
