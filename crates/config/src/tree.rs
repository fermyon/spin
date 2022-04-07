use std::collections::BTreeMap;
use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use crate::template::Template;
use crate::{Error, Key, Result};

/// A configuration tree.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Tree(BTreeMap<Path, Slot>);

impl Tree {
    pub(crate) fn get(&self, path: &Path) -> Result<&Slot> {
        self.0
            .get(path)
            .ok_or_else(|| Error::InvalidPath(format!("no slot at path {:?}", path)))
    }

    pub fn merge(&mut self, base: &Path, other: Tree) -> Result<()> {
        for (subpath, slot) in other.0.into_iter() {
            self.merge_slot(base + &subpath, slot)?;
        }
        Ok(())
    }

    pub fn merge_defaults(
        &mut self,
        base: &Path,
        defaults: impl IntoIterator<Item = (String, String)>,
    ) -> Result<()> {
        for (ref key, default) in defaults {
            let path = base + Key::new(key)?;
            let slot = Slot::from_default(default)?;
            self.merge_slot(path, slot)?;
        }
        Ok(())
    }

    fn merge_slot(&mut self, path: Path, slot: Slot) -> Result<()> {
        if self.0.contains_key(&path) {
            return Err(Error::InvalidPath(format!("duplicate key at {:?}", path)));
        }
        self.0.insert(path, slot);
        Ok(())
    }
}

/// A configuration path.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub struct Path(String);

impl Path {
    /// Creates a ConfigPath from a String.
    pub fn new(path: impl Into<String>) -> Result<Self> {
        let path = path.into();
        if path.is_empty() {
            return Err(Error::InvalidPath("empty".to_string()));
        }
        path.split('.').try_for_each(Key::validate)?;
        Ok(Path(path))
    }

    /// Returns the number of keys in this Path.
    pub fn size(&self) -> usize {
        self.0.matches('.').count() + 1
    }

    /// Resolves the given relative path (starting with at least one '.').
    pub fn resolve_relative(&self, rel: &str) -> Result<Self> {
        if rel.is_empty() {
            return Err(Error::InvalidPath("rel may not be empty".to_string()));
        }
        let key = rel.trim_start_matches('.');
        let dots = rel.len() - key.len();
        if dots == 0 {
            return Err(Error::InvalidPath("rel must start with a '.'".to_string()));
        }
        // Remove last `dots` components from path.
        let path = match self.0.rmatch_indices('.').chain([(0, "")]).nth(dots - 1) {
            Some((0, _)) => key.to_string(),
            Some((idx, _)) => format!("{}.{}", &self.0[..idx], key),
            None => {
                return Err(Error::InvalidPath(format!(
                    "rel has too many dots relative to base path {:?}",
                    self
                )))
            }
        };
        Ok(Self(path))
    }

    /// Produces an iterator over the keys of the path.
    pub fn keys(&self) -> impl Iterator<Item = Key<'_>> {
        self.0.split('.').map(Key)
    }
}

impl AsRef<str> for Path {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::ops::Add for &Path {
    type Output = Path;
    fn add(self, rhs: &Path) -> Self::Output {
        Path(format!("{}.{}", self.0, rhs.0))
    }
}

impl std::ops::Add<Key<'_>> for &Path {
    type Output = Path;
    fn add(self, key: Key) -> Self::Output {
        Path(format!("{}.{}", self.0, key.0))
    }
}

impl TryFrom<String> for Path {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

#[derive(Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(into = "RawSlot", try_from = "RawSlot")]
pub(crate) struct Slot {
    pub secret: bool,
    pub default: Option<Template>,
}

impl Slot {
    fn from_default(default: impl Into<Box<str>>) -> Result<Self> {
        Ok(Self {
            default: Some(Template::new(default)?),
            ..Default::default()
        })
    }
}

impl TryFrom<RawSlot> for Slot {
    type Error = Error;

    fn try_from(raw: RawSlot) -> Result<Self> {
        let default = match raw.default {
            Some(default) => Some(Template::new(default)?),
            None if !raw.required => {
                return Err(Error::InvalidSchema(
                    "slot must have a default if not required".to_string(),
                ));
            }
            None => None,
        };
        Ok(Self {
            default,
            secret: raw.secret,
        })
    }
}

impl From<Slot> for RawSlot {
    fn from(slot: Slot) -> Self {
        RawSlot {
            default: slot.default.as_ref().map(|tmpl| tmpl.to_string()),
            required: slot.default.is_none(),
            secret: slot.secret,
        }
    }
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let default = match self.default.as_ref() {
            Some(_) if self.secret => Some("<SECRET>".to_string()),
            not_secret => Some(format!("{:?}", not_secret)),
        };
        f.debug_struct("Slot")
            .field("secret", &self.secret)
            .field("default", &default)
            .finish()
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RawSection(pub HashMap<String, RawSlot>);

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct RawSlot {
    pub default: Option<String>,
    pub required: bool,
    pub secret: bool,
}

#[cfg(test)]
mod tests {
    use toml::toml;

    use super::*;

    #[test]
    fn paths_good() {
        for path in ["x", "x.y", "a.b_c.d", "f.a1.x_1"] {
            Path::new(path).expect(path);
        }
    }

    #[test]
    fn paths_bad() {
        for path in ["", "_x", "a._x", "a..b"] {
            Path::new(path).expect_err(path);
        }
    }

    #[test]
    fn path_keys() {
        assert_eq!(
            Path::new("a").unwrap().keys().collect::<Vec<_>>(),
            &[Key("a")]
        );
        assert_eq!(
            Path::new("a.b_c.d").unwrap().keys().collect::<Vec<_>>(),
            &[Key("a"), Key("b_c"), Key("d")]
        );
    }

    #[test]
    fn path_resolve_relative() {
        let path = Path::new("a.b.c").unwrap();
        for (rel, expected) in [(".x", "a.b.x"), ("..x", "a.x"), ("...x", "x")] {
            assert_eq!(path.resolve_relative(rel).unwrap().as_ref(), expected);
        }
    }

    #[test]
    fn path_resolve_relative_bad() {
        let path = Path::new("a.b.c").unwrap();
        for rel in ["", "x", "....x"] {
            path.resolve_relative(rel).expect_err(rel);
        }
    }

    #[test]
    fn slot_debug_secret() {
        let mut slot = Slot {
            default: Some(Template::new("sesame").unwrap()),
            ..Default::default()
        };
        assert!(format!("{:?}", slot).contains("sesame"));

        slot.secret = true;
        assert!(!format!("{:?}", slot).contains("sesame"));
        assert!(format!("{:?}", slot).contains("<SECRET>"));
    }

    #[test]
    fn tree_from_toml() {
        let tree: Tree = toml! {
            required_key = { required = true }
            secret_default = { default = "TOP-SECRET", secret = true }
        }
        .try_into()
        .unwrap();

        for (key, expected_slot) in [
            (
                "required_key",
                Slot {
                    default: None,
                    ..Default::default()
                },
            ),
            (
                "secret_default",
                Slot {
                    default: Some(Template::new("TOP-SECRET").unwrap()),
                    secret: true,
                },
            ),
        ] {
            let path = Path::new(key).expect(key);
            assert_eq!(tree.get(&path).expect(key), &expected_slot);
        }
    }

    #[test]
    fn invalid_slot() {
        toml! {
            not_required_or_default = { secret = true }
        }
        .try_into::<Tree>()
        .expect_err("should fail");
    }
}
