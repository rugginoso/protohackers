use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Username(String);

impl Username {
    pub fn inner(&self) -> &str {
        &self.0
    }
}

impl FromStr for Username {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            Err(Error::EmptyUsername)
        } else if !s.chars().all(char::is_alphanumeric) {
            Err(Error::AlphanumericOnly)
        } else {
            Ok(Self(s.to_owned()))
        }
    }
}

impl Display for Username {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("username cannot be empty")]
    EmptyUsername,
    #[error("username must be contain only alphanumeric characters and underscores")]
    AlphanumericOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_username() {
        assert!("validUser123".parse::<Username>().is_ok())
    }

    #[test]
    fn test_empty_username() {
        let result = "".parse::<Username>();
        assert!(matches!(result, Err(Error::EmptyUsername)));
    }

    #[test]
    fn test_invalid_username() {
        let result = "invalid user!".parse::<Username>();
        assert!(matches!(result, Err(Error::AlphanumericOnly)));
    }

    #[test]
    fn test_inner() {
        let username = "testUser".parse::<Username>().unwrap();
        assert_eq!(username.inner(), "testUser");
    }
}
