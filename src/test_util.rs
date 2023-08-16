//! Utilities for tests!

use crate::config::{
    Application, Config, Name, NativeCommand, Profile, ProfileReference,
    SideEffect, SideEffectCommand, ValueSource, ValueSourceInner,
    ValueSourceKind,
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
            multiple: false,
        })
    }
}

/// Shorthand for creating a native side effect
impl<const N: usize> From<[&str; N]> for SideEffectCommand {
    fn from(value: [&str; N]) -> Self {
        Self::Native(
            value
                .into_iter()
                .map(String::from)
                .collect::<Vec<String>>()
                .try_into()
                .unwrap(),
        )
    }
}

/// Shorthand for creating a shell side effect
impl From<&str> for SideEffectCommand {
    fn from(value: &str) -> Self {
        Self::Shell(value.into())
    }
}

// Builder-like functions to make it easy to create value sources
impl ValueSource {
    pub fn sensitive(mut self) -> Self {
        self.0.sensitive = true;
        self
    }

    pub fn multiple(mut self) -> Self {
        self.0.multiple = true;
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

/// Helper to create a native command
pub fn native<const N: usize>(
    program: &str,
    arguments: [&str; N],
) -> ValueSource {
    ValueSourceKind::NativeCommand {
        command: NativeCommand {
            program: program.into(),
            arguments: arguments.into_iter().map(String::from).collect(),
        },
    }
    .into()
}

/// Helper to create a shell command
pub fn shell(command: &str) -> ValueSource {
    ValueSourceKind::ShellCommand {
        command: command.into(),
    }
    .into()
}

/// Create a side effect from (setup, teardown)
pub fn side_effect<S: Into<SideEffectCommand>, T: Into<SideEffectCommand>>(
    setup: S,
    teardown: T,
) -> SideEffect {
    SideEffect {
        setup: Some(setup.into()),
        teardown: Some(teardown.into()),
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
