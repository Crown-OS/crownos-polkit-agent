//! What the daemon hands a dialog process on its stdin. D-Bus strings and user names never hold
//! NUL, so the fields are NUL-separated: `message`, `icon_name`, `cookie`, then one or more users.

use thiserror::Error;

const SEPARATOR: char = '\0';

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RequestError {
    #[error("the request is missing its {0}")]
    MissingField(&'static str),
    #[error("a request field holds a NUL byte")]
    EmbeddedNul,
}

/// One authentication the dialog shows: the polkit message, its icon, the cookie the helper
/// answers to, and the users who may authenticate, preferred first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogRequest {
    pub message: String,
    pub icon_name: String,
    pub cookie: String,
    pub users: Vec<String>,
}

impl DialogRequest {
    pub fn encode(&self) -> Result<String, RequestError> {
        let fields = [&self.message, &self.icon_name, &self.cookie]
            .into_iter()
            .chain(&self.users);
        let mut encoded = String::new();
        for (index, field) in fields.enumerate() {
            if field.contains(SEPARATOR) {
                return Err(RequestError::EmbeddedNul);
            }
            if index > 0 {
                encoded.push(SEPARATOR);
            }
            encoded.push_str(field);
        }
        Ok(encoded)
    }

    pub fn decode(encoded: &str) -> Result<Self, RequestError> {
        let mut fields = encoded.split(SEPARATOR).map(str::to_owned);
        let mut next = |name| fields.next().ok_or(RequestError::MissingField(name));
        let request = Self {
            message: next("message")?,
            icon_name: next("icon name")?,
            cookie: next("cookie")?,
            users: Vec::new(),
        };
        let users: Vec<String> = fields.filter(|user| !user.is_empty()).collect();
        if users.is_empty() {
            return Err(RequestError::MissingField("users"));
        }
        Ok(Self { users, ..request })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DialogRequest {
        DialogRequest {
            message: "Authentication is required to mount\nthe drive".into(),
            icon_name: "drive-harddisk".into(),
            cookie: "3-abc-1".into(),
            users: vec!["alice".into(), "root".into()],
        }
    }

    #[test]
    fn round_trips() {
        let encoded = sample().encode().unwrap();
        assert_eq!(DialogRequest::decode(&encoded).unwrap(), sample());
    }

    #[test]
    fn rejects_requests_without_users() {
        let request = DialogRequest {
            users: Vec::new(),
            ..sample()
        };
        let encoded = request.encode().unwrap();
        assert_eq!(
            DialogRequest::decode(&encoded),
            Err(RequestError::MissingField("users"))
        );
    }

    #[test]
    fn rejects_embedded_nul() {
        let request = DialogRequest {
            cookie: "a\0b".into(),
            ..sample()
        };
        assert_eq!(request.encode(), Err(RequestError::EmbeddedNul));
    }
}
