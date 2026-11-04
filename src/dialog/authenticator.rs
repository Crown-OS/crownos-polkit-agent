use std::future::Future;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::auth::{AuthError, Echo, HelperLink, HelperMessage};

/// What the dialog asks of the authenticator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// The answer to the current or next prompt.
    Answer(String),
    /// Abandons the attempt and starts over as this user.
    SwitchUser(String),
}

/// What the authenticator tells the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    Prompt {
        text: String,
        echo: Echo,
    },
    Error(String),
    Info(String),
    /// The answer was wrong; a fresh attempt is already running.
    Rejected,
    /// The helper could not run; the next command retries.
    Unavailable(String),
    Authenticated,
}

enum Ending {
    Authenticated,
    Rejected,
    SwitchUser(String),
    Closed,
}

/// Runs helper attempts as `user` until one succeeds or the dialog goes away: a rejected answer
/// starts a new attempt, a user switch abandons the current one.
pub async fn authenticate<L, F>(
    mut start_helper: impl FnMut(String) -> F,
    mut user: String,
    updates: UnboundedSender<Update>,
    mut commands: UnboundedReceiver<Command>,
) where
    L: HelperLink,
    F: Future<Output = Result<L, AuthError>>,
{
    let mut early_answer = None;
    loop {
        let ending = match start_helper(user.clone()).await {
            Ok(helper) => converse(helper, &mut early_answer, &updates, &mut commands).await,
            Err(error) => Err(error),
        };
        match ending {
            Ok(Ending::Authenticated) => {
                _ = updates.send(Update::Authenticated);
                return;
            }
            Ok(Ending::Rejected) => _ = updates.send(Update::Rejected),
            Ok(Ending::SwitchUser(name)) => user = name,
            Ok(Ending::Closed) => return,
            Err(error) => {
                _ = updates.send(Update::Unavailable(error.to_string()));
                match commands.recv().await {
                    Some(Command::Answer(answer)) => early_answer = Some(answer),
                    Some(Command::SwitchUser(name)) => user = name,
                    None => return,
                }
            }
        }
    }
}

async fn converse(
    mut helper: impl HelperLink,
    early_answer: &mut Option<String>,
    updates: &UnboundedSender<Update>,
    commands: &mut UnboundedReceiver<Command>,
) -> Result<Ending, AuthError> {
    let mut awaiting_answer = false;
    let mut answered = false;
    loop {
        tokio::select! {
            biased;
            command = commands.recv() => match command {
                Some(Command::Answer(answer)) if awaiting_answer => {
                    awaiting_answer = false;
                    helper.send_line(&answer).await?;
                    answered = true;
                }
                Some(Command::Answer(answer)) => *early_answer = Some(answer),
                Some(Command::SwitchUser(name)) => return Ok(Ending::SwitchUser(name)),
                None => return Ok(Ending::Closed),
            },
            message = helper.next_message() => match message? {
                HelperMessage::Prompt { text, echo } => match early_answer.take() {
                    Some(answer) => {
                        helper.send_line(&answer).await?;
                        answered = true;
                    }
                    None => {
                        awaiting_answer = true;
                        _ = updates.send(Update::Prompt { text, echo });
                    }
                },
                HelperMessage::Error(text) => _ = updates.send(Update::Error(text)),
                HelperMessage::Info(text) => _ = updates.send(Update::Info(text)),
                HelperMessage::Success => return Ok(Ending::Authenticated),
                HelperMessage::Failure if answered => return Ok(Ending::Rejected),
                HelperMessage::Failure => return Err(AuthError::FailedBeforePrompt),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

    /// A scripted helper: each attempt plays its messages, waiting for an answer after prompts.
    struct FakeHelper {
        user: String,
        script: VecDeque<HelperMessage>,
        answers: UnboundedSender<(String, String)>,
        awaiting_answer: bool,
    }

    impl HelperLink for FakeHelper {
        async fn next_message(&mut self) -> Result<HelperMessage, AuthError> {
            if self.awaiting_answer {
                std::future::pending::<()>().await;
            }
            let message = self.script.pop_front().unwrap_or(HelperMessage::Failure);
            self.awaiting_answer = matches!(message, HelperMessage::Prompt { .. });
            Ok(message)
        }

        async fn send_line(&mut self, line: &str) -> Result<(), AuthError> {
            self.awaiting_answer = false;
            _ = self.answers.send((self.user.clone(), line.to_owned()));
            Ok(())
        }
    }

    fn password_prompt() -> HelperMessage {
        HelperMessage::Prompt {
            text: "Password: ".into(),
            echo: Echo::Hidden,
        }
    }

    /// Accepts only `alice` with `secret`, and fails to start for `ghost`.
    fn helper_for(
        answers: UnboundedSender<(String, String)>,
    ) -> impl FnMut(String) -> std::future::Ready<Result<FakeHelper, AuthError>> {
        let attempts = std::cell::Cell::new(0);
        move |user| {
            attempts.set(attempts.get() + 1);
            if user == "ghost" {
                return std::future::ready(Err(AuthError::HelperMissing));
            }
            let verdict = match (user.as_str(), attempts.get()) {
                ("alice", 1) => HelperMessage::Failure,
                _ => HelperMessage::Success,
            };
            std::future::ready(Ok(FakeHelper {
                user,
                script: VecDeque::from([password_prompt(), verdict]),
                answers: answers.clone(),
                awaiting_answer: false,
            }))
        }
    }

    #[tokio::test]
    async fn retries_after_a_rejected_answer() {
        let (answers, mut answered) = unbounded_channel();
        let (updates, mut updated) = unbounded_channel();
        let (commands, command_receiver) = unbounded_channel();
        commands.send(Command::Answer("wrong".into())).unwrap();
        let run = tokio::spawn(authenticate(
            helper_for(answers),
            "alice".into(),
            updates,
            command_receiver,
        ));

        assert_eq!(updated.recv().await, Some(Update::Rejected));
        assert!(matches!(updated.recv().await, Some(Update::Prompt { .. })));
        commands.send(Command::Answer("secret".into())).unwrap();
        assert_eq!(updated.recv().await, Some(Update::Authenticated));
        run.await.unwrap();

        assert_eq!(
            answered.recv().await,
            Some(("alice".into(), "wrong".into()))
        );
        assert_eq!(
            answered.recv().await,
            Some(("alice".into(), "secret".into()))
        );
    }

    #[tokio::test]
    async fn switching_users_restarts_the_attempt() {
        let (answers, mut answered) = unbounded_channel();
        let (updates, mut updated) = unbounded_channel();
        let (commands, command_receiver) = unbounded_channel();
        let run = tokio::spawn(authenticate(
            helper_for(answers),
            "ghost".into(),
            updates,
            command_receiver,
        ));

        assert!(matches!(updated.recv().await, Some(Update::Unavailable(_))));
        commands.send(Command::SwitchUser("bob".into())).unwrap();
        assert!(matches!(updated.recv().await, Some(Update::Prompt { .. })));
        commands.send(Command::Answer("pw".into())).unwrap();
        assert_eq!(updated.recv().await, Some(Update::Authenticated));
        run.await.unwrap();
        assert_eq!(answered.recv().await, Some(("bob".into(), "pw".into())));
    }

    #[tokio::test]
    async fn stops_when_the_dialog_closes() {
        let (answers, _answered) = unbounded_channel();
        let (updates, mut updated) = unbounded_channel();
        let (commands, command_receiver) = unbounded_channel::<Command>();
        let run = tokio::spawn(authenticate(
            helper_for(answers),
            "bob".into(),
            updates,
            command_receiver,
        ));
        assert!(matches!(updated.recv().await, Some(Update::Prompt { .. })));
        drop(commands);
        run.await.unwrap();
    }
}
