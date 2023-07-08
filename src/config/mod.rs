mod cereal;
mod merge;

use crate::config::merge::Merge;
use anyhow::{anyhow, Context};
use indexmap::{IndexMap, IndexSet};
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
    #[serde(default, alias = "vars")]
    pub variables: IndexMap<String, IndexSet<ValueSource>>,

    /// A set of named applications (as in, a use case, purpose, etc.). An
    /// application typically has one or more variables that control it, and
    /// each variable may multiple values to select between. Each value set
    /// is known as a "profile".
    #[serde(default, rename = "apps")]
    pub applications: IndexMap<String, Application>,
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
    pub variables: IndexMap<String, ValueSource>,
}

/// The source of an exported value. Can be a literal value or an embedded
/// command, which will be evaluated into a value lazily. A "value source" is
/// actually composed of 3 types:
/// - [ValueSource] - A newtype wrapper, which is required to customize
///   deserialization without entirely reimplementing it
/// - [ValueSourceInner] - Container for fields that are common among all value
///   sources
/// - [ValueSourceKind] - Enum that captures the different kinds of value
///   sources and the data that can vary between them
#[derive(Clone, Debug, Serialize, Eq, Hash, PartialEq)]
pub struct ValueSource(pub ValueSourceInner);

/// Main value source data structure. This holds the data that is common to all
/// value source kinds, plus the kind itself (which may hold additional
/// kind-specific data).
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct ValueSourceInner {
    #[serde(flatten)]
    pub kind: ValueSourceKind,
    #[serde(default)]
    pub sensitive: bool,
}

/// The various kinds of supported value sources. This will only hold data
/// that's specific to each kind.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
#[serde(tag = "type")]
pub enum ValueSourceKind {
    /// A plain string value
    #[serde(rename = "literal")]
    Literal { value: String },
    /// A native command (program+arguments) that will be executed at runtime
    /// to get the variable's value. Useful for values that change, secrets,
    /// etc.
    #[serde(rename = "command")]
    NativeCommand { command: NativeCommand },
    /// A command that will be executed via the shell. Allows access to
    /// aliases, pipes, etc.
    #[serde(rename = "shell")]
    ShellCommand { command: String },
    /// Run a command in a kubernetes pod.
    #[serde(rename = "kubernetes")]
    KubernetesCommand {
        /// Command to execute in the pod
        command: NativeCommand,
        /// Label query used to find the pod
        /// https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/
        pod_selector: String,
        /// Optional namespace to run in. If omitted, use current namespace
        /// from kubectl context.
        namespace: Option<String>,
        /// Optional container to run in. If omitted, use
        /// `kubectl.kubernetes.io/default-container` annotation.
        container: Option<String>,
    },
}

/// A native command is a program name/path, with zero or more arguments. This
/// is a separate type so we can easily serialize into/out of `Vec<String>`.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct NativeCommand {
    /// Name (or path) of program to execute
    pub program: String,
    /// Arguments (if any) to pass to the program
    pub arguments: Vec<String>,
}

impl From<NativeCommand> for Vec<String> {
    fn from(value: NativeCommand) -> Self {
        let mut elements = value.arguments;
        elements.insert(0, value.program); // O(n)! Spooky!
        elements
    }
}

impl TryFrom<Vec<String>> for NativeCommand {
    type Error = anyhow::Error;

    fn try_from(mut value: Vec<String>) -> Result<Self, Self::Error> {
        if !value.is_empty() {
            Ok(Self {
                program: value.remove(0),
                arguments: value,
            })
        } else {
            Err(anyhow!("Command array must have at least one element"))
        }
    }
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

impl ValueSource {
    /// Build a [ValueSource] from a simple string value. All extra fields
    /// are populated with defaults.
    pub fn from_literal(value: &str) -> Self {
        Self(ValueSourceInner {
            kind: ValueSourceKind::Literal {
                value: value.to_owned(),
            },
            sensitive: false,
        })
    }
}

impl Display for ValueSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0.kind {
            ValueSourceKind::Literal { value } => write!(f, "{value}"),
            ValueSourceKind::NativeCommand { command } => {
                write!(f, "{command} (native)")
            }
            ValueSourceKind::ShellCommand { command } => {
                write!(f, "`{command}` (shell)")
            }
            ValueSourceKind::KubernetesCommand {
                command,
                pod_selector,
                namespace,
                container,
            } => {
                write!(
                    f,
                    "{command} (kubernetes {}[{pod_selector}]",
                    namespace.as_deref().unwrap_or("<current namespace>")
                )?;
                if let Some(container) = container {
                    write!(f, ".{container}")?;
                }
                write!(f, ")")?;
                Ok(())
            }
        }
    }
}

impl Display for NativeCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}", self.program)?;
        for argument in &self.arguments {
            write!(f, " {argument}")?;
        }
        write!(f, "`")?;
        Ok(())
    }
}

// This makes it more ergonomic to call execute_native
impl<'a, S: Into<String>, I: IntoIterator<Item = &'a str>> From<(S, I)>
    for NativeCommand
{
    fn from((program, arguments): (S, I)) -> Self {
        Self {
            program: program.into(),
            arguments: arguments.into_iter().map(|s| s.to_owned()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Application, Config, Profile, ValueSourceKind};
    use indexmap::{IndexMap, IndexSet};
    use serde_test::{
        assert_de_tokens, assert_de_tokens_error, assert_tokens, Token,
    };

    const CONFIG: &str = r#"
[vars]
PASSWORD = [
    "hunter2",
    {type = "literal", value = "secret-but-not-really", sensitive = true},
    {type = "shell", command = "echo secret_password | base64", sensitive = true},
]
TEST_VARIABLE = ["abc", {type = "command", command = ["echo", "def"]}]

[apps.server]
dev = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
prd = {SERVICE1 = "prd", SERVICE2 = "also-prd"}
[apps.server.secret]
SERVICE1 = {type = "literal", value = "secret", sensitive = true}
SERVICE2 = {type = "command", command = ["echo", "also-secret"], sensitive = true}

[apps.empty]
    "#;

    /// Helper for building an IndexMap
    fn map<V, const N: usize>(
        items: [(&'static str, V); N],
    ) -> IndexMap<String, V> {
        items.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
    }

    /// Helper for building an IndexMap
    fn set<V: Hash + Eq, const N: usize>(items: [V; N]) -> IndexSet<V> {
        IndexSet::from(items)
    }

    /// Helper to create a non-sensitive literal
    fn literal(value: &str) -> ValueSource {
        ValueSource(ValueSourceInner {
            kind: ValueSourceKind::Literal {
                value: value.to_owned(),
            },
            sensitive: false,
        })
    }

    /// Helper to create a sensitive literal
    fn literal_sensitive(value: &str) -> ValueSource {
        ValueSource(ValueSourceInner {
            kind: ValueSourceKind::Literal {
                value: value.to_owned(),
            },
            sensitive: true,
        })
    }

    /// Helper to create a native command
    fn native<const N: usize>(
        program: &str,
        arguments: [&str; N],
        sensitive: bool,
    ) -> ValueSource {
        ValueSource(ValueSourceInner {
            kind: ValueSourceKind::NativeCommand {
                command: NativeCommand {
                    program: program.into(),
                    arguments: arguments
                        .into_iter()
                        .map(String::from)
                        .collect(),
                },
            },
            sensitive,
        })
    }

    /// Helper to create a shell command
    fn shell(command: &str, sensitive: bool) -> ValueSource {
        ValueSource(ValueSourceInner {
            kind: ValueSourceKind::ShellCommand {
                command: command.to_owned(),
            },
            sensitive,
        })
    }

    /// General catch-all test
    #[test]
    fn test_parse_config() {
        let expected = Config {
            variables: map([
                (
                    "TEST_VARIABLE",
                    IndexSet::from([
                        literal("abc"),
                        native("echo", ["def"], false),
                    ]),
                ),
                (
                    "PASSWORD",
                    IndexSet::from([
                        literal("hunter2"),
                        literal_sensitive("secret-but-not-really"),
                        shell("echo secret_password | base64", true),
                    ]),
                ),
            ]),

            applications: map([
                (
                    "server",
                    Application {
                        profiles: map([
                            (
                                "dev",
                                Profile {
                                    variables: map([
                                        ("SERVICE1", literal("dev")),
                                        ("SERVICE2", literal("also-dev")),
                                    ]),
                                },
                            ),
                            (
                                "prd",
                                Profile {
                                    variables: map([
                                        ("SERVICE1", literal("prd")),
                                        ("SERVICE2", literal("also-prd")),
                                    ]),
                                },
                            ),
                            (
                                "secret",
                                Profile {
                                    variables: map([
                                        (
                                            "SERVICE1",
                                            literal_sensitive("secret"),
                                        ),
                                        (
                                            "SERVICE2",
                                            native(
                                                "echo",
                                                ["also-secret"],
                                                true,
                                            ),
                                        ),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "empty",
                    Application {
                        profiles: IndexMap::new(),
                    },
                ),
            ]),
        };
        assert_eq!(toml::from_str::<Config>(CONFIG).unwrap(), expected);
    }

    #[test]
    fn test_parse_literal() {
        // Flat or complex syntax (they're equivalent)
        assert_de_tokens(&literal("abc"), &[Token::Str("abc")]);
        // This is the serialized format
        assert_tokens(
            &literal_sensitive("abc").0,
            &[
                Token::Map { len: None },
                Token::Str("type"),
                Token::Str("literal"),
                Token::Str("value"),
                Token::Str("abc"),
                Token::Str("sensitive"),
                Token::Bool(true),
                Token::MapEnd,
            ],
        );

        // Can't parse non-strings
        // https://github.com/LucasPickering/env-select/issues/16
        assert_de_tokens_error::<ValueSource>(
            &[Token::I32(16)],
            "invalid type: integer `16`, expected string or map",
        );
        assert_de_tokens_error::<ValueSource>(
            &[Token::Bool(true)],
            "invalid type: boolean `true`, expected string or map",
        );
    }

    #[test]
    fn test_parse_native_command() {
        // Default native command
        assert_de_tokens(
            &native("echo", ["test"], false),
            &[
                Token::Map { len: None },
                Token::Str("type"),
                Token::Str("command"),
                Token::Str("command"),
                Token::Seq { len: Some(2) },
                Token::Str("echo"),
                Token::Str("test"),
                Token::SeqEnd,
                Token::MapEnd,
            ],
        );

        // Sensitive native command
        assert_tokens(
            &native("echo", ["test"], true).0,
            &[
                Token::Map { len: None },
                Token::Str("type"),
                Token::Str("command"),
                Token::Str("command"),
                Token::Seq { len: Some(2) },
                Token::Str("echo"),
                Token::Str("test"),
                Token::SeqEnd,
                Token::Str("sensitive"),
                Token::Bool(true),
                Token::MapEnd,
            ],
        );

        // Empty command - error
        assert_de_tokens_error::<ValueSourceKind>(
            &[
                Token::Map { len: None },
                Token::Str("type"),
                Token::Str("command"),
                Token::Str("command"),
                Token::Seq { len: Some(0) },
                Token::SeqEnd,
                Token::MapEnd,
            ],
            "Command array must have at least one element",
        );
    }

    #[test]
    fn test_parse_shell_command() {
        // Regular shell command
        assert_de_tokens(
            &shell("echo test", false),
            &[
                Token::Map { len: None },
                Token::Str("type"),
                Token::Str("shell"),
                Token::Str("command"),
                Token::Str("echo test"),
                Token::MapEnd,
            ],
        );

        // Sensitive shell command
        assert_tokens(
            &shell("echo test", true).0,
            &[
                Token::Map { len: None },
                Token::Str("type"),
                Token::Str("shell"),
                Token::Str("command"),
                Token::Str("echo test"),
                Token::Str("sensitive"),
                Token::Bool(true),
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn test_parse_kubernetes() {
        assert_tokens(
            &ValueSourceInner {
                kind: ValueSourceKind::KubernetesCommand {
                    command: NativeCommand {
                        program: "printenv".to_owned(),
                        arguments: vec!["DB_PASSWORD".to_owned()],
                    },
                    pod_selector: "app=api".to_owned(),
                    namespace: Some("development".to_owned()),
                    container: Some("main".to_owned()),
                },
                sensitive: true,
            },
            &[
                Token::Map { len: None },
                Token::Str("type"),
                Token::Str("kubernetes"),
                //
                Token::Str("command"),
                Token::Seq { len: Some(2) },
                Token::Str("printenv"),
                Token::Str("DB_PASSWORD"),
                Token::SeqEnd,
                //
                Token::Str("pod_selector"),
                Token::Str("app=api"),
                //
                Token::Str("namespace"),
                Token::Some,
                Token::Str("development"),
                //
                Token::Str("container"),
                Token::Some,
                Token::Str("main"),
                //
                Token::Str("sensitive"),
                Token::Bool(true),
                Token::MapEnd,
            ],
        );
    }

    #[test]
    fn test_parse_unknown_type() {
        assert_de_tokens_error::<ValueSource>(
            &[
                Token::Map { len: None },
                Token::Str("type"),
                Token::Str("unknown"),
                Token::MapEnd,
            ],
            "unknown variant `unknown`, expected one of \
            `literal`, `command`, `shell`, `kubernetes`",
        )
    }

    #[test]
    fn test_set_merge() {
        let mut v1 = set([1]);
        let v2 = set([2, 1]);
        v1.merge(v2);
        assert_eq!(v1, set([1, 2]));
    }

    #[test]
    fn test_map_merge() {
        let mut map1 = map([("a", set([1])), ("b", set([2]))]);
        let map2 = map([("a", set([3])), ("c", set([4]))]);
        map1.merge(map2);
        assert_eq!(
            map1,
            map([("a", set([1, 3])), ("b", set([2])), ("c", set([4])),])
        );
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = Config {
            variables: map([
                ("VAR1", set([literal("val1"), literal("val2")])),
                ("VAR2", set([literal("val1")])),
            ]),
            applications: map([
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "prof1",
                                Profile {
                                    variables: map([
                                        // Gets overwritten
                                        ("VAR1", literal("val1")),
                                        ("VAR2", literal("val2")),
                                    ]),
                                },
                            ),
                            // No conflict
                            (
                                "prof2",
                                Profile {
                                    variables: map([
                                        ("VAR1", literal("val11")),
                                        ("VAR2", literal("val22")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                // No conflict
                (
                    "app2",
                    Application {
                        profiles: map([(
                            "prof1",
                            Profile {
                                variables: map([("VAR1", literal("val1"))]),
                            },
                        )]),
                    },
                ),
            ]),
        };
        let config2 = Config {
            variables: map([("VAR1", set([literal("val3")]))]),
            applications: map([
                // Merged into existing
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "prof1",
                                Profile {
                                    variables: map([
                                        // Overwrites
                                        ("VAR1", literal("val7")),
                                    ]),
                                },
                            ),
                            // No conflict
                            (
                                "prof3",
                                Profile {
                                    variables: map([
                                        ("VAR1", literal("val111")),
                                        ("VAR2", literal("val222")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                // No conflict
                (
                    "app3",
                    Application {
                        profiles: map([(
                            "prof1",
                            Profile {
                                variables: map([("VAR1", literal("val11"))]),
                            },
                        )]),
                    },
                ),
            ]),
        };
        config1.merge(config2);
        assert_eq!(
            config1,
            Config {
                variables: map([
                    (
                        "VAR1",
                        set([
                            literal("val1"),
                            literal("val2"),
                            literal("val3")
                        ])
                    ),
                    ("VAR2", set([literal("val1")])),
                ]),
                applications: map([
                    (
                        "app1",
                        Application {
                            profiles: map([
                                (
                                    "prof1",
                                    Profile {
                                        variables: map([
                                            ("VAR1", literal("val7")),
                                            ("VAR2", literal("val2")),
                                        ])
                                    }
                                ),
                                (
                                    "prof2",
                                    Profile {
                                        variables: map([
                                            ("VAR1", literal("val11")),
                                            ("VAR2", literal("val22"))
                                        ])
                                    }
                                ),
                                (
                                    "prof3",
                                    Profile {
                                        variables: map([
                                            ("VAR1", literal("val111")),
                                            ("VAR2", literal("val222")),
                                        ])
                                    }
                                ),
                            ]),
                        }
                    ),
                    (
                        "app2",
                        Application {
                            profiles: map([(
                                "prof1",
                                Profile {
                                    variables: map([("VAR1", literal("val1"))])
                                },
                            )]),
                        }
                    ),
                    (
                        "app3",
                        Application {
                            profiles: map([(
                                "prof1",
                                Profile {
                                    variables: map([(
                                        "VAR1",
                                        literal("val11"),
                                    )])
                                },
                            )]),
                        }
                    ),
                ]),
            }
        );
    }
}
