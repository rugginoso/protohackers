use crate::username::Username;
use std::collections::BTreeSet;

#[derive(Default)]
pub struct State {
    users: BTreeSet<Username>,
}

impl State {
    pub fn add_user(&mut self, username: Username) -> Result<Vec<Username>, Error> {
        let already_connected_users: Vec<Username> = self.users.iter().cloned().collect();
        if already_connected_users.contains(&username) {
            Err(Error::UsernameAlreadyTaken)
        } else {
            self.users.insert(username);
            Ok(already_connected_users)
        }
    }

    pub fn remove_user(&mut self, username: &Username) {
        self.users.remove(username);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("username already taken")]
    UsernameAlreadyTaken,
}

#[cfg(test)]
mod tests {
    use crate::state::{Error, State};

    #[test]
    fn test_add_user() {
        let mut state = State::default();

        let user1 = "user1".parse().unwrap();
        let already_connected_users = state.add_user(user1).unwrap();
        assert!(already_connected_users.is_empty());

        let user2 = "user2".parse().unwrap();
        let already_connected_users = state.add_user(user2).unwrap();
        assert_eq!(already_connected_users, vec!["user1".parse().unwrap()]);
    }

    #[test]
    fn test_add_user_username_already_taken() {
        let mut state = State::default();

        let user1 = "user1".parse().unwrap();
        state.add_user(user1).unwrap();

        assert!(matches!(
            state.add_user("user1".parse().unwrap()),
            Err(Error::UsernameAlreadyTaken)
        ));
    }
}
