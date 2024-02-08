//! Utilities for tests!

use crate::config::{
    Application, Config, MultiVariable, Name, Profile, ProfileReference,
    SideEffect, ValueSource, ValueSourceInner, ValueSourceKind,
};
use indexmap::{IndexMap, IndexSet};
use rstest_reuse::{self, *};
use std::{hash::Hash, path::Path};

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<&str> for ProfileReference {
    fn from(value: &str) -> Self {
        value.parse().expect("Invalid profile reference")
    }
}

impl From<ValueSourceKind> for ValueSource {
    fn from(kind: ValueSourceKind) -> Self {
        Self(ValueSourceInner {
            kind,
            sensitive: false,
            multiple: false.into(),
        })
    }
}

// Builder-like functions to make it easy to create value sources
impl ValueSource {
    pub fn sensitive(mut self) -> Self {
        self.0.sensitive = true;
        self
    }

    pub fn multiple(mut self) -> Self {
        self.0.multiple = true.into();
        self
    }

    pub fn multiple_filtered(mut self, values: &[&str]) -> Self {
        self.0.multiple = MultiVariable::List(
            values.iter().copied().map(String::from).collect(),
        );
        self
    }

    pub fn cwd(mut self, cwd: &str) -> Self {
        match &mut self.0.kind {
            ValueSourceKind::Command { cwd: dest, .. } => {
                *dest = Some(cwd.into())
            }
            _ => unimplemented!(),
        }
        self
    }
}

/// Helper to create a full config, from a mapping of applications and profiles
pub fn config(applications: Vec<(&str, Vec<(&str, Profile)>)>) -> Config {
    Config {
        applications: applications
            .into_iter()
            .map(|(name, profiles)| {
                (
                    (*name).into(),
                    Application {
                        profiles: profiles
                            .into_iter()
                            .map(|(name, profile)| ((*name).into(), profile))
                            .collect(),
                    },
                )
            })
            .collect(),
    }
}

/// Helper for building an IndexMap
pub fn map<'a, K: Eq + Hash + PartialEq + From<&'a str>, V, const N: usize>(
    items: [(&'a str, V); N],
) -> IndexMap<K, V> {
    items.into_iter().map(|(k, v)| (k.into(), v)).collect()
}

/// Helper for building an IndexSet
pub fn set<'a, V: From<&'a str> + Hash + Eq, const N: usize>(
    items: [&'a str; N],
) -> IndexSet<V> {
    items.into_iter().map(V::from).collect()
}

/// Helper to create a non-sensitive literal
pub fn literal(value: &str) -> ValueSource {
    ValueSourceKind::Literal {
        value: value.to_owned(),
    }
    .into()
}

/// Helper to create a file value source
pub fn file(path: impl AsRef<Path>) -> ValueSource {
    ValueSourceKind::File {
        path: path.as_ref().to_owned(),
    }
    .into()
}

/// Helper to create a shell command
pub fn command(command: &str) -> ValueSource {
    ValueSourceKind::Command {
        command: command.to_owned().into(),
        cwd: None,
    }
    .into()
}

/// Create a side effect from (setup, teardown)
pub fn side_effect(setup: &str, teardown: &str) -> SideEffect {
    SideEffect {
        setup: Some(setup.to_owned().into()),
        teardown: Some(teardown.to_owned().into()),
    }
}

/// Test template to run test with all shells
#[template]
#[rstest]
pub fn all_shells(
    #[values(ShellKind::Bash, ShellKind::Zsh, ShellKind::Fish)]
    shell_kind: ShellKind,
) {
}
