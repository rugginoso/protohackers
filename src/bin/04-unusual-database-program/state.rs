use std::{borrow::Cow, collections::HashMap, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(Cow<'static, str>);

impl From<String> for Key {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl From<&'static str> for Key {
    fn from(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value(Cow<'static, str>);

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl From<&'static str> for Value {
    fn from(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

const VERSION_KEY: Key = Key(Cow::Borrowed("version"));
const VERSION_VALUE: Value = Value(Cow::Borrowed("Ken's Key-Value Store 1.0"));

#[derive(Default)]
pub struct State {
    data: HashMap<Key, Value>,
}

impl State {
    pub fn insert(&mut self, key: Key, value: Value) {
        if key != VERSION_KEY {
            self.data.insert(key, value);
        }
    }

    pub fn retrieve(&self, key: &Key) -> Option<&Value> {
        if key == &VERSION_KEY {
            Some(&VERSION_VALUE)
        } else {
            self.data.get(key)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::state::{Key, State, VERSION_KEY, VERSION_VALUE, Value};

    #[test]
    fn test_retrieve_inserted_value() {
        let mut state = State::default();

        state.insert(Key::from("foo"), Value::from("bar"));

        assert_eq!(state.retrieve(&Key::from("foo")), Some(&Value::from("bar")));
    }

    #[test]
    fn test_retrieve_not_inserted_value() {
        let mut state = State::default();

        state.insert(Key::from("foo"), Value::from("bar"));

        assert!(state.retrieve(&Key::from("baz")).is_none());
    }

    #[test]
    fn test_retrieve_updated_value() {
        let mut state = State::default();

        state.insert(Key::from("foo"), Value::from("bar"));
        state.insert(Key::from("foo"), Value::from("baz"));

        assert_eq!(state.retrieve(&Key::from("foo")), Some(&Value::from("baz")));
    }

    #[test]
    fn test_retrieve_version() {
        let state = State::default();

        assert_eq!(state.retrieve(&VERSION_KEY), Some(&VERSION_VALUE));
    }

    #[test]
    fn test_retrieve_updated_version() {
        let mut state = State::default();

        state.insert(VERSION_KEY, Value::from("updated version"));

        assert_eq!(state.retrieve(&VERSION_KEY), Some(&VERSION_VALUE));
    }
}
