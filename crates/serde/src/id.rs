//! ID (de)serialization

use serde::{Deserialize, Serialize};

/// An ID is a non-empty string containing one or more component model
/// `word`s separated by a delimiter char.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct Id<const DELIM: char>(String);

impl<const DELIM: char> std::fmt::Display for Id<DELIM> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<const DELIM: char> AsRef<str> for Id<DELIM> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<const DELIM: char> From<Id<DELIM>> for String {
    fn from(value: Id<DELIM>) -> Self {
        value.0
    }
}

impl<const DELIM: char> TryFrom<String> for Id<DELIM> {
    type Error = String;

    fn try_from(id: String) -> Result<Self, Self::Error> {
        if id.is_empty() {
            return Err("empty".into());
        }
        for word in id.split(DELIM) {
            if word.is_empty() {
                return Err(format!("{DELIM:?}-separated words must not be empty"));
            }
            let mut chars = word.chars();
            let first = chars.next().unwrap();
            if !first.is_ascii_alphabetic() {
                return Err(format!(
                    "{DELIM:?}-separated words must start with an ASCII letter; got {first:?}"
                ));
            }
            let word_is_uppercase = first.is_ascii_uppercase();
            for ch in chars {
                if ch.is_ascii_digit() {
                } else if !ch.is_ascii_alphanumeric() {
                    return Err(format!(
                        "{DELIM:?}-separated words may only contain alphanumeric ASCII; got {ch:?}"
                    ));
                } else if ch.is_ascii_uppercase() != word_is_uppercase {
                    return Err(format!("{DELIM:?}-separated words must be all lowercase or all UPPERCASE; got {word:?}"));
                }
            }
        }
        Ok(Self(id))
    }
}
