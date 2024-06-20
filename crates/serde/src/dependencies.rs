//! Types for working with component dependencies.

use crate::KebabId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_pkg_common::package::PackageRef;

/// Name of an import dependency.
///
/// For example: `foo:bar/baz@0.1.0`, `foo:bar/baz`, `foo:bar@0.1.0`, `foo:bar`
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(into = "String", try_from = "String")]
pub struct DependencyName {
    /// The package spec, `foo:bar`, `foo:bar@0.1.0`.
    pub package: PackageRef,
    /// Package version
    pub version: Option<semver::Version>,
    /// Optional interface name.
    pub interface: Option<KebabId>,
}

// TODO: replace with derive once wasm-pkg-common is released
impl PartialOrd for DependencyName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// TODO: replace with derive once wasm-pkg-common is released
impl Ord for DependencyName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let big_ole_tup = (
            self.package.namespace().as_ref(),
            self.package.name().as_ref(),
            self.interface.as_ref(),
            self.version.as_ref(),
        );
        let other_big_ole_tup = (
            other.package.namespace().as_ref(),
            other.package.name().as_ref(),
            other.interface.as_ref(),
            other.version.as_ref(),
        );
        big_ole_tup.cmp(&other_big_ole_tup)
    }
}

impl std::fmt::Display for DependencyName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.package)?;
        if let Some(interface) = &self.interface {
            write!(f, "/{interface}")?;
        }
        if let Some(version) = &self.version {
            write!(f, "@{version}")?;
        }
        Ok(())
    }
}

impl TryFrom<String> for DependencyName {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<DependencyName> for String {
    fn from(value: DependencyName) -> Self {
        value.to_string()
    }
}

impl FromStr for DependencyName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, version) = match s.split_once('@') {
            Some((name, version)) => (name, Some(version.parse()?)),
            None => (s, None),
        };

        let (package, interface) = match name.split_once('/') {
            Some((package, interface)) => (
                package.parse()?,
                Some(
                    interface
                        .to_string()
                        .try_into()
                        .map_err(|e| anyhow::anyhow!("{e}"))?,
                ),
            ),
            None => (name.parse()?, None),
        };

        Ok(Self {
            package,
            version,
            interface,
        })
    }
}
