use std::error::Error;
use std::fmt::{self, Display, Formatter};

const MAX_USERNAME_LENGTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalIdentityService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityError {
    InvalidUsername,
}

impl Display for IdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUsername => formatter.write_str(
                "username must contain 1-32 ASCII letters, digits, underscores, dots, or hyphens",
            ),
        }
    }
}

impl Error for IdentityError {}

impl LocalIdentityService {
    /// Authenticates a development-only local identity.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::InvalidUsername`] when the supplied username is
    /// empty, too long, or contains characters outside the local identity policy.
    pub fn authenticate(self, username: &str) -> Result<Account, IdentityError> {
        if username.is_empty()
            || username.len() > MAX_USERNAME_LENGTH
            || !username
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        {
            return Err(IdentityError::InvalidUsername);
        }

        Ok(Account {
            id: format!("local:{}", username.to_ascii_lowercase()),
            username: username.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{IdentityError, LocalIdentityService};

    #[test]
    fn valid_local_user_receives_one_operator() {
        let service = LocalIdentityService;
        let account = service
            .authenticate("Echo-Runner")
            .expect("local identity should authenticate");

        assert_eq!(account.id, "local:echo-runner");
        assert_eq!(account.username, "Echo-Runner");
    }

    #[test]
    fn unsafe_username_is_rejected() {
        let result = LocalIdentityService.authenticate("bad user\n");

        assert_eq!(result, Err(IdentityError::InvalidUsername));
    }
}
