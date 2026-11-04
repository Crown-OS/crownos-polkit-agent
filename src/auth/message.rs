use super::AuthError;
use crate::util::c_unescape::c_unescape;

/// Whether the helper's prompt wants its answer shown while typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Echo {
    Hidden,
    Visible,
}

/// One line the helper writes, after GLib unescaping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperMessage {
    Prompt { text: String, echo: Echo },
    Error(String),
    Info(String),
    Success,
    Failure,
}

impl HelperMessage {
    pub fn parse(line: &str) -> Result<Self, AuthError> {
        let line = c_unescape(line.strip_suffix('\n').unwrap_or(line));
        let prompt = |text: &str, echo| Self::Prompt {
            text: text.to_owned(),
            echo,
        };
        if let Some(text) = line.strip_prefix("PAM_PROMPT_ECHO_OFF ") {
            Ok(prompt(text, Echo::Hidden))
        } else if let Some(text) = line.strip_prefix("PAM_PROMPT_ECHO_ON ") {
            Ok(prompt(text, Echo::Visible))
        } else if let Some(text) = line.strip_prefix("PAM_ERROR_MSG ") {
            Ok(Self::Error(text.to_owned()))
        } else if let Some(text) = line.strip_prefix("PAM_TEXT_INFO ") {
            Ok(Self::Info(text.to_owned()))
        } else if line.starts_with("SUCCESS") {
            Ok(Self::Success)
        } else if line.starts_with("FAILURE") {
            Ok(Self::Failure)
        } else {
            Err(AuthError::UnexpectedLine(line.into_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_helper_line() {
        let cases = [
            (
                "PAM_PROMPT_ECHO_OFF Password: \n",
                HelperMessage::Prompt {
                    text: "Password: ".into(),
                    echo: Echo::Hidden,
                },
            ),
            (
                "PAM_PROMPT_ECHO_ON login:",
                HelperMessage::Prompt {
                    text: "login:".into(),
                    echo: Echo::Visible,
                },
            ),
            (
                r"PAM_ERROR_MSG Wrong\tpassword",
                HelperMessage::Error("Wrong\tpassword".into()),
            ),
            (
                r"PAM_TEXT_INFO Touch the key\n",
                HelperMessage::Info("Touch the key\n".into()),
            ),
            ("SUCCESS\n", HelperMessage::Success),
            ("FAILURE", HelperMessage::Failure),
        ];
        for (line, expected) in cases {
            assert_eq!(HelperMessage::parse(line).unwrap(), expected, "{line:?}");
        }
    }

    #[test]
    fn rejects_unknown_lines() {
        assert!(matches!(
            HelperMessage::parse("polkit-agent-helper-1: needs to be setuid root"),
            Err(AuthError::UnexpectedLine(_))
        ));
    }
}
