use crownui::prelude::{Cx, Signal};

use super::{authenticator::Update, shake::shake};
use crate::auth::Echo;

const DEFAULT_PROMPT: &str = "Password";
const REJECTED_NOTICE: &str = "That didn't work. Please try again.";

/// The line under the password field.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Feedback {
    #[default]
    None,
    Info(String),
    Error(String),
}

impl Feedback {
    pub fn text(&self) -> &str {
        match self {
            Self::None => "",
            Self::Info(text) | Self::Error(text) => text,
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

/// Everything the dialog shows that changes while it is open.
#[derive(Clone, Copy)]
pub struct DialogState {
    pub users: Signal<Vec<String>>,
    pub selected_user: Signal<Option<usize>>,
    pub answer: Signal<String>,
    pub prompt: Signal<String>,
    pub echo: Signal<Echo>,
    pub feedback: Signal<Feedback>,
    pub verifying: Signal<bool>,
    pub shake_offset: Signal<f32>,
}

impl DialogState {
    pub fn new(cx: &mut Cx, users: Vec<String>) -> Self {
        Self {
            users: cx.signal(users),
            selected_user: cx.signal(Some(0)),
            answer: cx.signal(String::new()),
            prompt: cx.signal(DEFAULT_PROMPT.to_owned()),
            echo: cx.signal(Echo::Hidden),
            feedback: cx.signal(Feedback::None),
            verifying: cx.signal(false),
            shake_offset: cx.signal(0.0),
        }
    }

    /// Takes the typed answer for the helper, unless one is already being checked.
    pub fn take_answer(self, cx: &mut Cx) -> Option<String> {
        if self.verifying.get(cx) {
            return None;
        }
        let answer = self.answer.get(cx);
        self.answer.set(cx, String::new());
        self.verifying.set(cx, true);
        self.feedback.set(cx, Feedback::None);
        Some(answer)
    }

    /// The chosen user's name, after clearing what was typed for the previous one.
    pub fn switch_user(self, cx: &mut Cx) -> Option<String> {
        self.answer.set(cx, String::new());
        self.verifying.set(cx, false);
        self.feedback.set(cx, Feedback::None);
        let index = self.selected_user.get(cx)?;
        self.users.with(cx, |users| users.get(index).cloned())
    }

    pub fn apply(self, cx: &mut Cx, update: Update) {
        match update {
            Update::Prompt { text, echo } => {
                self.prompt.set(cx, prompt_label(&text));
                self.echo.set(cx, echo);
                self.verifying.set(cx, false);
            }
            Update::Info(text) => self.feedback.set(cx, Feedback::Info(text)),
            Update::Error(text) => self.feedback.set(cx, Feedback::Error(text)),
            Update::Rejected => {
                if !self.feedback.with(cx, Feedback::is_error) {
                    self.feedback
                        .set(cx, Feedback::Error(REJECTED_NOTICE.to_owned()));
                }
                self.verifying.set(cx, false);
                shake(cx, self.shake_offset);
            }
            Update::Unavailable(reason) => {
                self.feedback.set(cx, Feedback::Error(reason));
                self.verifying.set(cx, false);
            }
            Update::Authenticated => self.verifying.set(cx, true),
        }
    }
}

/// PAM's `"Password: "` as a field label.
fn prompt_label(prompt: &str) -> String {
    let label = prompt.trim_end().trim_end_matches(':').trim_end();
    if label.is_empty() {
        DEFAULT_PROMPT.to_owned()
    } else {
        label.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_pam_prompts() {
        assert_eq!(prompt_label("Password: "), "Password");
        assert_eq!(prompt_label("Verification code:"), "Verification code");
        assert_eq!(prompt_label(": "), DEFAULT_PROMPT);
    }
}
