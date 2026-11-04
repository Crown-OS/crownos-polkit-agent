use std::future::Future;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, Lines};

use super::{AuthError, HelperMessage};

/// Anything that speaks the helper's line protocol: a helper, or a fake one in tests.
pub trait HelperLink: Send {
    /// The next message; a helper that hangs up without a verdict has failed. Cancel-safe.
    fn next_message(&mut self) -> impl Future<Output = Result<HelperMessage, AuthError>> + Send;

    fn send_line(&mut self, line: &str) -> impl Future<Output = Result<(), AuthError>> + Send;
}

/// The line protocol with the helper: it writes messages, the agent answers prompts one line each.
pub struct Conversation<R, W> {
    lines: Lines<R>,
    writer: W,
}

impl<R: AsyncBufRead, W> Conversation<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            lines: reader.lines(),
            writer,
        }
    }
}

impl<R, W> HelperLink for Conversation<R, W>
where
    R: AsyncBufRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    async fn next_message(&mut self) -> Result<HelperMessage, AuthError> {
        match self.lines.next_line().await? {
            Some(line) => HelperMessage::parse(&line),
            None => Ok(HelperMessage::Failure),
        }
    }

    async fn send_line(&mut self, line: &str) -> Result<(), AuthError> {
        if line.contains('\n') {
            return Err(AuthError::MultilineAnswer);
        }
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{BufReader, duplex, split};

    use super::*;
    use crate::auth::Echo;

    #[tokio::test]
    async fn answers_a_prompt_until_the_verdict() {
        let (agent_end, helper_end) = duplex(256);
        let (agent_reader, agent_writer) = split(agent_end);
        let mut conversation = Conversation::new(BufReader::new(agent_reader), agent_writer);

        let fake_helper = tokio::spawn(async move {
            let (reader, mut writer) = split(helper_end);
            let mut lines = BufReader::new(reader).lines();
            let cookie = lines.next_line().await.unwrap();
            writer
                .write_all(b"PAM_PROMPT_ECHO_OFF Password: \n")
                .await
                .unwrap();
            let password = lines.next_line().await.unwrap();
            writer.write_all(b"SUCCESS\n").await.unwrap();
            (cookie, password)
        });

        conversation.send_line("cookie-1").await.unwrap();
        assert_eq!(
            conversation.next_message().await.unwrap(),
            HelperMessage::Prompt {
                text: "Password: ".into(),
                echo: Echo::Hidden
            }
        );
        conversation.send_line("hunter2").await.unwrap();
        assert_eq!(
            conversation.next_message().await.unwrap(),
            HelperMessage::Success
        );

        let (cookie, password) = fake_helper.await.unwrap();
        assert_eq!(cookie.as_deref(), Some("cookie-1"));
        assert_eq!(password.as_deref(), Some("hunter2"));
    }

    #[tokio::test]
    async fn a_hang_up_is_a_failure() {
        let (agent_end, helper_end) = duplex(64);
        drop(helper_end);
        let (reader, writer) = split(agent_end);
        let mut conversation = Conversation::new(BufReader::new(reader), writer);
        assert_eq!(
            conversation.next_message().await.unwrap(),
            HelperMessage::Failure
        );
    }

    #[tokio::test]
    async fn refuses_multiline_answers() {
        let (agent_end, _helper_end) = duplex(64);
        let (reader, writer) = split(agent_end);
        let mut conversation = Conversation::new(BufReader::new(reader), writer);
        assert!(matches!(
            conversation.send_line("a\nb").await,
            Err(AuthError::MultilineAnswer)
        ));
    }
}
