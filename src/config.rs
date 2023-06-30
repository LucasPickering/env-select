use anyhow::{anyhow, Context};
use indexmap::{map::Entry, IndexMap, IndexSet};
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
};

const FILE_NAME: &str = ".env-select.toml";

/// Add configuration, as loaded from one or more config files. We use
/// [indexmap::IndexMap] in here to preserve ordering from the input files.
/// This (hopefully) makes usage more intuitive for the use.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct Config {
    /// A set of possible values for individual variables. Each variable maps
    /// to zero or more possible values, and the user can select from this
    /// list for each variable *independently* of the other variables. We use
    /// an ordered set here so the ordering from the user's file(s) is
    /// maintained, but without duplicates.
    #[serde(default, rename = "vars")]
    pub variables: IndexMap<String, IndexSet<Value>>,

    /// A set of named applications (as in, a use case, purpose, etc.). An
    /// application typically has one or more variables that control it, and
    /// each variable may multiple values to select between. Each value set
    /// is known as a "profile".
    #[serde(default, rename = "apps")]
    pub applications: IndexMap<String, Application>,
}

impl Config {
    /// Load config from the current directory and all parents. Any config
    /// file in any directory in the hierarchy will be loaded and merged into
    /// the config, with lower files take precedence.
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Config::default();

        // Iterate *backwards*, so that we go top->bottom in the dir tree.
        // Lower files should have higher precedence.
        for path in Self::get_all_files()?.iter().rev() {
            debug!("Loading config from file {path:?}");
            let content = fs::read_to_string(path)
                .with_context(|| format!("Error reading file {path:?}"))?;
            match toml::from_str(&content) {
                Ok(parsed) => {
                    trace!("Loaded from file {path:?}: {parsed:?}");
                    config.merge(parsed);
                }
                Err(error) => {
                    error!("{path:?} will be ignored due to error: {error}")
                }
            }
        }

        debug!("Loaded config: {config:#?}");
        Ok(config)
    }

    /// Build an error that contains a suggestion of all available variables and
    /// profiles
    pub fn get_suggestion_error(&self, message: &str) -> anyhow::Error {
        anyhow!(
            "{} Try one of the following:
    Variables: {}
    Applications: {}",
            message,
            self.variables
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            self.applications
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    /// Starting at the current directory, walk *up* the tree and collect the
    /// list of all config files.
    fn get_all_files() -> anyhow::Result<Vec<PathBuf>> {
        let cwd = env::current_dir()?;

        let mut config_files: Vec<PathBuf> = Vec::new();
        let mut search_dir: Option<&Path> = Some(&cwd);
        // Walk *up* the tree until we've hit the root
        while let Some(dir) = search_dir {
            trace!("Scanning for config file in {dir:?}");
            let path = dir.join(FILE_NAME);
            if path.exists() {
                trace!("Found config file at {path:?}");
                config_files.push(path);
            }
            search_dir = dir.parent();
        }

        Ok(config_files)
    }
}

/// An application is a grouping of profiles. Each profile should be different
/// "versions" of the same "application", e.g. dev vs prd for the same service.
/// Different colors of the same car, so to speak.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct Application {
    #[serde(flatten)]
    pub profiles: IndexMap<String, Profile>,
}

/// A profile is a set of fixed variable mappings, i.e. each variable maps to
/// a singular value.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct Profile {
    #[serde(flatten)]
    pub variables: IndexMap<String, Value>,
}

/// A variable's value. Can be a literal value, or an embedded command, which
/// will be evaluated into a value lazily.
///
/// The variant structure here is important for deserialization. A single string
/// should be interpreted as a literal, because that's the most common case.
/// An object should be treated as other variants, based on the field structure.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
#[serde(untagged)]
// TODO rename to ValueSource?
pub enum Value {
    /// A plain string value
    Literal(String),
    /// A command that will be executed at runtime to get the variable's value.
    /// Useful for values that change, secrets, etc.
    Command { command: String },
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Literal(value) => write!(f, "{value}"),
            Value::Command { command } => write!(f, "`{command}`"),
        }
    }
}

/// Indicates that two values of this type can be merged together.
trait Merge {
    /// Merge another value into this one. The "other" value **will take
    /// precedence** over this one, meaning conflicting values from the incoming
    /// will overwrite.
    fn merge(&mut self, other: Self);
}

impl Merge for Config {
    fn merge(&mut self, other: Self) {
        self.variables.merge(other.variables);
        self.applications.merge(other.applications);
    }
}

impl Merge for Application {
    fn merge(&mut self, other: Self) {
        self.profiles.merge(other.profiles)
    }
}

impl Merge for Profile {
    fn merge(&mut self, other: Self) {
        // Incoming entries take priority over ours
        self.variables.extend(other.variables.into_iter())
    }
}

impl<T: Eq + Hash> Merge for IndexSet<T> {
    fn merge(&mut self, other: Self) {
        self.extend(other)
    }
}

impl<K: Eq + Hash, V: Merge> Merge for IndexMap<K, V> {
    fn merge(&mut self, other: Self) {
        for (k, other_v) in other {
            match self.entry(k) {
                Entry::Occupied(mut entry) => entry.get_mut().merge(other_v),
                Entry::Vacant(entry) => {
                    entry.insert(other_v);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::{indexmap, indexset};

    impl From<&str> for Value {
        fn from(s: &str) -> Self {
            Value::Literal(s.into())
        }
    }

    #[test]
    fn test_set_merge() {
        let mut v1 = indexset! {1};
        let v2 = indexset! {2, 1};
        v1.merge(v2);
        assert_eq!(v1, indexset! {1, 2});
    }

    #[test]
    fn test_map_merge() {
        let mut map1 = indexmap! {
            "a" => indexset!{1},
            "b" => indexset!{2},
        };
        let map2 = indexmap! {
            "a" => indexset!{3},
            "c" => indexset!{4},
        };
        map1.merge(map2);
        assert_eq!(
            map1,
            indexmap! {
                "a" => indexset!{1,3},
                "b" => indexset!{2},
                "c" => indexset!{4},
            }
        );
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = Config {
            variables: indexmap! {
                "VAR1".into() => indexset!{"val1".into(), "val2".into()},
                "VAR2".into() => indexset!{"val1".into()},
            },
            applications: indexmap! {
                "app1".into() => Application {
                    profiles: indexmap! {
                        "prof1".into() => Profile {
                            variables: indexmap! {
                                // Gets overwritten
                                "VAR1".into() => "val1".into(),
                                "VAR2".into() => "val2".into(),
                            }
                        },
                        // No conflict
                        "prof2".into() => Profile {
                            variables: indexmap! {
                                "VAR1".into() => "val11".into(),
                                "VAR2".into() => "val22".into(),
                            }
                        },
                    },
                },
                // No conflict
                "app2".into() => Application {
                    profiles: indexmap! {
                        "prof1".into() => Profile {
                            variables: indexmap! {
                                "VAR1".into() => "val1".into(),
                            }
                        },
                    },
                },
            },
        };
        let config2 = Config {
            variables: indexmap! {
                "VAR1".into() => indexset!{"val3".into()},
            },
            applications: indexmap! {
                // Merged into existing
                "app1".into() => Application {
                    profiles: indexmap! {
                        "prof1".into() => Profile {
                            variables: indexmap! {
                                // Overwrites
                                "VAR1".into() => "val7".into(),
                            }
                        },
                        // No conflict
                        "prof3".into() => Profile {
                            variables: indexmap! {
                                "VAR1".into() => "val111".into(),
                                "VAR2".into() => "val222".into(),
                            }
                        },
                    },
                },
                // No conflict
                "app3".into() => Application {
                    profiles: indexmap! {
                        "prof1".into() => Profile {
                            variables: indexmap! {
                                "VAR1".into() => "val11".into(),
                            }
                        },
                    },
                },
            },
        };
        config1.merge(config2);
        assert_eq!(
            config1,
            Config {
                variables: indexmap! {
                    "VAR1".into() => indexset!{"val1".into(), "val2".into(), "val3".into()},
                    "VAR2".into() => indexset!{"val1".into()},
                },
                applications: indexmap! {
                    "app1".into() => Application {
                        profiles: indexmap! {
                            "prof1".into() => Profile {
                                variables: indexmap! {
                                    "VAR1".into() => "val7".into(),
                                    "VAR2".into() => "val2".into(),
                                }
                            },
                            "prof2".into() => Profile {
                                variables: indexmap! {
                                    "VAR1".into() => "val11".into(),
                                    "VAR2".into() => "val22".into(),
                                }
                            },
                            "prof3".into() => Profile {
                                variables: indexmap! {
                                    "VAR1".into() => "val111".into(),
                                    "VAR2".into() => "val222".into(),
                                }
                            },
                        },
                    },
                    "app2".into() => Application {
                        profiles: indexmap! {
                            "prof1".into() => Profile {
                                variables: indexmap! {
                                    "VAR1".into() => "val1".into(),
                                }
                            },
                        },
                    },
                    "app3".into() => Application {
                        profiles: indexmap! {
                            "prof1".into() => Profile {
                                variables: indexmap! {
                                    "VAR1".into() => "val11".into(),
                                }
                            },
                        },
                    },
                },
            }
        );
    }
}
