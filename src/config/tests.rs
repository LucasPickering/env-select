use super::*;
use crate::config::{Application, Config, Profile, ValueSourceKind};
use indexmap::{IndexMap, IndexSet};
use pretty_assertions::assert_eq;
use serde_test::{
    assert_de_tokens, assert_de_tokens_error, assert_tokens, Token,
};

/// A general config to test parsing. This doesn't include all edge cases, but
/// it's got a good variety
const CONFIG: &str = r#"
[applications.base.profiles.base.variables]
I_AM_HERE = "true"

[applications.server.profiles.base]
extends = ["base/base"]
variables = {USERNAME = "user"}
[applications.server.profiles.dev]
extends = ["base"]
variables = {SERVICE1 = "dev", SERVICE2 = "also-dev"}
[applications.server.profiles.prd]
extends = ["base"]
variables = {SERVICE1 = "prd", SERVICE2 = "also-prd"}

[applications.server.profiles.secret]
extends = ["base"]
[applications.server.profiles.secret.variables]
SERVICE1 = {type = "literal", value = "secret", sensitive = true}
SERVICE2 = {type = "command", command = ["echo", "also-secret"], sensitive = true}
SERVICE3 = {type = "shell", command = "echo secret_password | base64", sensitive = true}

[applications.empty]
"#;

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
        })
    }
}

impl ValueSource {
    fn sensitive(mut self, sensitive: bool) -> Self {
        self.0.sensitive = sensitive;
        self
    }
}

/// Helper for building an IndexMap
fn map<'a, K: Eq + Hash + PartialEq + From<&'a str>, V, const N: usize>(
    items: [(&'a str, V); N],
) -> IndexMap<K, V> {
    items.into_iter().map(|(k, v)| (k.into(), v)).collect()
}

/// Helper for building an IndexSet
fn set<'a, V: From<&'a str> + Hash + Eq, const N: usize>(
    items: [&'a str; N],
) -> IndexSet<V> {
    items.into_iter().map(V::from).collect()
}

/// Helper to create a non-sensitive literal
fn literal(value: &str) -> ValueSource {
    ValueSourceKind::Literal {
        value: value.to_owned(),
    }
    .into()
}

/// Helper to create a native command
fn native<const N: usize>(program: &str, arguments: [&str; N]) -> ValueSource {
    ValueSourceKind::NativeCommand {
        command: NativeCommand {
            program: program.into(),
            arguments: arguments.into_iter().map(String::from).collect(),
        },
    }
    .into()
}

/// Helper to create a shell command
fn shell(command: &str) -> ValueSource {
    ValueSourceKind::ShellCommand {
        command: command.to_owned(),
    }
    .into()
}

/// General catch-all test
#[test]
fn test_parse_config() {
    let expected = Config {
        applications: map([
            (
                "base",
                Application {
                    profiles: map([(
                        "base",
                        Profile {
                            extends: set([]),
                            variables: map([("I_AM_HERE", literal("true"))]),
                        },
                    )]),
                },
            ),
            (
                "server",
                Application {
                    profiles: map([
                        (
                            "base",
                            Profile {
                                extends: set(["base/base"]),
                                variables: map([("USERNAME", literal("user"))]),
                            },
                        ),
                        (
                            "dev",
                            Profile {
                                extends: set(["base"]),
                                variables: map([
                                    ("SERVICE1", literal("dev")),
                                    ("SERVICE2", literal("also-dev")),
                                ]),
                            },
                        ),
                        (
                            "prd",
                            Profile {
                                extends: set(["base"]),
                                variables: map([
                                    ("SERVICE1", literal("prd")),
                                    ("SERVICE2", literal("also-prd")),
                                ]),
                            },
                        ),
                        (
                            "secret",
                            Profile {
                                extends: set(["base"]),
                                variables: map([
                                    (
                                        "SERVICE1",
                                        literal("secret").sensitive(true),
                                    ),
                                    (
                                        "SERVICE2",
                                        native("echo", ["also-secret"])
                                            .sensitive(true),
                                    ),
                                    (
                                        "SERVICE3",
                                        shell("echo secret_password | base64")
                                            .sensitive(true),
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
fn test_parse_name() {
    assert_tokens(
        &Name("-123_valid-name with\nwhitespace _".to_string()).0,
        &[Token::Str("-123_valid-name with\nwhitespace _")],
    );

    // Invalid names
    assert_de_tokens_error::<Name>(
        &[Token::Str("")],
        "Invalid name: empty string",
    );
    assert_de_tokens_error::<Name>(
        &[Token::Str(" ")],
        "Invalid name: contains leading/trailing whitespace",
    );
    assert_de_tokens_error::<Name>(
        &[Token::Str(" name")],
        "Invalid name: contains leading/trailing whitespace",
    );
    assert_de_tokens_error::<Name>(
        &[Token::Str("name ")],
        "Invalid name: contains leading/trailing whitespace",
    );
    assert_de_tokens_error::<Name>(
        &[Token::Str("/")],
        "Invalid name: contains one of reserved characters \\/*?!",
    );
}

#[test]
fn test_parse_profile_reference() {
    // Just profile name
    assert_tokens(
        &ProfileReference {
            application: None,
            profile: Name("profile".to_string()),
        },
        &[Token::Str("profile")],
    );
    // Application+profile
    assert_tokens(
        &ProfileReference {
            application: Some(Name("app".to_string())),
            profile: Name("prof".to_string()),
        },
        &[Token::Str("app/prof")],
    );

    assert_de_tokens_error::<ProfileReference>(
        &[Token::Str("")],
        "Invalid name: empty string",
    );
    assert_de_tokens_error::<ProfileReference>(
        &[Token::Str("/")],
        "Invalid name: empty string",
    );
    assert_de_tokens_error::<ProfileReference>(
        &[Token::Str("/prof")],
        "Invalid name: empty string",
    );
    assert_de_tokens_error::<ProfileReference>(
        &[Token::Str("app/")],
        "Invalid name: empty string",
    );
    assert_de_tokens_error::<ProfileReference>(
        &[Token::Str("app/prof/")],
        "Invalid name: contains one of reserved characters \\/*?!",
    );
    assert_de_tokens_error::<ProfileReference>(
        &[Token::Str("/app/prof")],
        "Invalid name: empty string",
    );
    assert_de_tokens_error::<ProfileReference>(
        &[Token::Str("app//prof")],
        "Invalid name: contains one of reserved characters \\/*?!",
    );
}

#[test]
fn test_parse_literal() {
    // Flat or complex syntax (they're equivalent)
    assert_de_tokens(&literal("abc"), &[Token::Str("abc")]);
    // This is the serialized format
    assert_tokens(
        &literal("abc").sensitive(true).0,
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
        &native("echo", ["test"]),
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
        &native("echo", ["test"]).sensitive(true).0,
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
        &shell("echo test"),
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
        &shell("echo test").sensitive(true).0,
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
        &ValueSource::from(ValueSourceKind::KubernetesCommand {
            command: NativeCommand {
                program: "printenv".to_owned(),
                arguments: vec!["DB_PASSWORD".to_owned()],
            },
            pod_selector: "app=api".to_owned(),
            namespace: Some("development".to_owned()),
            container: Some("main".to_owned()),
        })
        .sensitive(true)
        .0,
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
fn test_merge_map() {
    let mut map1: IndexMap<String, IndexSet<&str>> =
        map([("a", set(["1"])), ("b", set(["2"]))]);
    let map2 = map([("a", set(["3"])), ("c", set(["4"]))]);
    map1.merge(map2);
    assert_eq!(
        map1,
        map([("a", set(["1", "3"])), ("b", set(["2"])), ("c", set(["4"])),])
    );
}

#[test]
fn test_merge_config() {
    let mut config1 = Config {
        applications: map([
            (
                "app1",
                Application {
                    profiles: map([
                        (
                            "prof1",
                            Profile {
                                extends: set(["prof2"]),
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
                                extends: set([]),
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
                            extends: set([]),
                            variables: map([("VAR1", literal("val1"))]),
                        },
                    )]),
                },
            ),
        ]),
    };
    let config2 = Config {
        applications: map([
            // Merged into existing
            (
                "app1",
                Application {
                    profiles: map([
                        (
                            "prof1",
                            Profile {
                                extends: set(["prof3"]),
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
                                extends: set([]),
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
                            extends: set([]),
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
            applications: map([
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "prof1",
                                Profile {
                                    extends: set(["prof2", "prof3"]),
                                    variables: map([
                                        ("VAR1", literal("val7")),
                                        ("VAR2", literal("val2")),
                                    ])
                                }
                            ),
                            (
                                "prof2",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("VAR1", literal("val11")),
                                        ("VAR2", literal("val22"))
                                    ])
                                }
                            ),
                            (
                                "prof3",
                                Profile {
                                    extends: set([]),
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
                                extends: set([]),
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
                                extends: set([]),
                                variables: map([("VAR1", literal("val11"))])
                            },
                        )]),
                    }
                ),
            ]),
        }
    );
}

#[test]
fn test_inherit_single() {
    let mut config = Config {
        applications: map([(
            "app",
            Application {
                profiles: map([
                    (
                        "base",
                        Profile {
                            extends: set([]),
                            variables: map([
                                ("VAR1", literal("base")),
                                ("VAR2", literal("base")),
                            ]),
                        },
                    ),
                    (
                        "child",
                        Profile {
                            extends: set(["base"]),
                            variables: map([
                                ("VAR1", literal("child")),
                                // VAR2 comes from base
                                ("VAR3", literal("child")),
                            ]),
                        },
                    ),
                ]),
            },
        )]),
    };
    config
        .resolve_inheritance()
        .expect("Error resolving valid inheritance");
    assert_eq!(
        config,
        Config {
            applications: map([(
                "app",
                Application {
                    profiles: map([
                        (
                            "base",
                            Profile {
                                extends: set([]),
                                variables: map([
                                    ("VAR1", literal("base")),
                                    ("VAR2", literal("base")),
                                ]),
                            },
                        ),
                        (
                            "child",
                            Profile {
                                extends: set(["base"]),
                                variables: map([
                                    ("VAR1", literal("child")),
                                    ("VAR3", literal("child")),
                                    ("VAR2", literal("base")),
                                ]),
                            },
                        ),
                    ]),
                },
            ),]),
        }
    );
}

#[test]
fn test_inherit_linear() {
    let mut config = Config {
        applications: map([
            (
                "app1",
                Application {
                    profiles: map([
                        (
                            "base",
                            Profile {
                                extends: set([]),
                                variables: map([
                                    ("VAR1", literal("base")),
                                    ("VAR2", literal("base")),
                                ]),
                            },
                        ),
                        (
                            "child1",
                            Profile {
                                extends: set(["base"]),
                                variables: map([
                                    ("VAR1", literal("child1")),
                                    // VAR2 comes from base
                                    ("VAR3", literal("child1")),
                                ]),
                            },
                        ),
                    ]),
                },
            ),
            (
                "app2",
                Application {
                    profiles: map([(
                        "child2",
                        Profile {
                            extends: set(["app1/child1"]),
                            variables: map([
                                ("VAR1", literal("child2")),
                                // VAR2 comes from base
                                // VAR3 comes from child1
                                ("VAR4", literal("child2")),
                            ]),
                        },
                    )]),
                },
            ),
        ]),
    };
    config
        .resolve_inheritance()
        .expect("Error resolving valid inheritance");
    assert_eq!(
        config,
        Config {
            applications: map([
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "base",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("VAR1", literal("base")),
                                        ("VAR2", literal("base")),
                                    ]),
                                },
                            ),
                            (
                                "child1",
                                Profile {
                                    extends: set(["base"]),
                                    variables: map([
                                        ("VAR1", literal("child1")),
                                        ("VAR3", literal("child1")),
                                        ("VAR2", literal("base")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "app2",
                    Application {
                        profiles: map([(
                            "child2",
                            Profile {
                                extends: set(["app1/child1"]),
                                variables: map([
                                    ("VAR1", literal("child2")),
                                    ("VAR4", literal("child2")),
                                    ("VAR3", literal("child1")),
                                    ("VAR2", literal("base")),
                                ]),
                            },
                        )]),
                    },
                ),
            ]),
        }
    );
}

#[test]
fn test_inherit_nonlinear() {
    let mut config = Config {
        applications: map([
            (
                "app1",
                Application {
                    profiles: map([
                        (
                            "base1",
                            Profile {
                                extends: set([]),
                                variables: map([
                                    ("BASE_VAR1", literal("base1")),
                                    ("BASE_VAR2", literal("base1")),
                                ]),
                            },
                        ),
                        (
                            "prof2",
                            Profile {
                                extends: set(["base1"]),
                                variables: map([
                                    ("BASE_VAR2", literal("prof2")),
                                    ("CHILD_VAR1", literal("prof2")),
                                ]),
                            },
                        ),
                        (
                            "base2",
                            Profile {
                                extends: set([]),
                                variables: map([
                                    ("BASE_VAR3", literal("base2")),
                                    ("BASE_VAR4", literal("base2")),
                                ]),
                            },
                        ),
                    ]),
                },
            ),
            (
                "app2",
                Application {
                    profiles: map([
                        (
                            "prof1",
                            Profile {
                                extends: set(["app1/base1"]),
                                variables: map([(
                                    "BASE_VAR2",
                                    literal("prof1"),
                                )]),
                            },
                        ),
                        (
                            "prof3",
                            Profile {
                                extends: set(["app2/prof1", "app1/prof2"]),
                                variables: map([
                                    ("CHILD_VAR2", literal("prof3")),
                                    ("CHILD_VAR3", literal("prof3")),
                                ]),
                            },
                        ),
                        (
                            "prof4",
                            Profile {
                                extends: set(["prof1"]),
                                variables: map([
                                    ("CHILD_VAR4", literal("prof4")),
                                    ("BASE_VAR4", literal("prof4")),
                                ]),
                            },
                        ),
                        (
                            "prof5",
                            Profile {
                                extends: set(["prof4", "prof3", "app1/base2"]),
                                variables: map([(
                                    "CHILD_VAR5",
                                    literal("prof5"),
                                )]),
                            },
                        ),
                    ]),
                },
            ),
            (
                "app3",
                Application {
                    profiles: map([
                        (
                            "solo",
                            Profile {
                                extends: set([]),
                                variables: map([(
                                    "SOLO_VAR1",
                                    literal("solo1"),
                                )]),
                            },
                        ),
                        (
                            "striker1",
                            Profile {
                                extends: set([]),
                                variables: map([
                                    ("CHILD_VAR1", literal("striker1")),
                                    ("CHILD_VAR2", literal("striker1")),
                                ]),
                            },
                        ),
                        (
                            "striker2",
                            Profile {
                                extends: set(["striker1"]),
                                variables: map([
                                    ("CHILD_VAR1", literal("striker2")),
                                    ("CHILD_VAR3", literal("striker2")),
                                ]),
                            },
                        ),
                    ]),
                },
            ),
        ]),
    };
    config
        .resolve_inheritance()
        .expect("Error resolving valid inheritance");
    assert_eq!(
        config,
        Config {
            applications: map([
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "base1",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("BASE_VAR1", literal("base1")),
                                        ("BASE_VAR2", literal("base1")),
                                    ]),
                                },
                            ),
                            (
                                "prof2",
                                Profile {
                                    extends: set(["base1"]),
                                    variables: map([
                                        ("BASE_VAR2", literal("prof2")),
                                        ("CHILD_VAR1", literal("prof2")),
                                        // base1
                                        ("BASE_VAR1", literal("base1")),
                                    ]),
                                },
                            ),
                            (
                                "base2",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("BASE_VAR3", literal("base2")),
                                        ("BASE_VAR4", literal("base2")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "app2",
                    Application {
                        profiles: map([
                            (
                                "prof1",
                                Profile {
                                    extends: set(["app1/base1"]),
                                    variables: map([
                                        ("BASE_VAR2", literal("prof1")),
                                        ("BASE_VAR1", literal("base1")),
                                    ]),
                                },
                            ),
                            (
                                "prof3",
                                Profile {
                                    extends: set(["app2/prof1", "app1/prof2"]),
                                    variables: map([
                                        ("CHILD_VAR2", literal("prof3")),
                                        ("CHILD_VAR3", literal("prof3")),
                                        // prof1
                                        ("BASE_VAR1", literal("base1")),
                                        ("BASE_VAR2", literal("prof1")),
                                        // prof2
                                        ("CHILD_VAR1", literal("prof2")),
                                    ]),
                                },
                            ),
                            (
                                "prof4",
                                Profile {
                                    extends: set(["prof1"]),
                                    variables: map([
                                        ("CHILD_VAR4", literal("prof4")),
                                        ("BASE_VAR4", literal("prof4")),
                                        // prof1
                                        ("BASE_VAR2", literal("prof1")),
                                        ("BASE_VAR1", literal("base1")),
                                    ]),
                                },
                            ),
                            (
                                "prof5",
                                Profile {
                                    extends: set([
                                        "prof4",
                                        "prof3",
                                        "app1/base2",
                                    ]),
                                    variables: map([
                                        ("CHILD_VAR5", literal("prof5"),),
                                        // prof4
                                        ("CHILD_VAR4", literal("prof4")),
                                        ("BASE_VAR4", literal("prof4")),
                                        ("BASE_VAR2", literal("prof1")),
                                        ("BASE_VAR1", literal("base1")),
                                        // prof3
                                        ("CHILD_VAR2", literal("prof3")),
                                        ("CHILD_VAR3", literal("prof3")),
                                        ("CHILD_VAR1", literal("prof2")),
                                        // base2
                                        ("BASE_VAR3", literal("base2")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "app3",
                    Application {
                        profiles: map([
                            (
                                "solo",
                                Profile {
                                    extends: set([]),
                                    variables: map([(
                                        "SOLO_VAR1",
                                        literal("solo1"),
                                    )]),
                                },
                            ),
                            (
                                "striker1",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("CHILD_VAR1", literal("striker1")),
                                        ("CHILD_VAR2", literal("striker1")),
                                    ]),
                                },
                            ),
                            (
                                "striker2",
                                Profile {
                                    extends: set(["striker1"]),
                                    variables: map([
                                        ("CHILD_VAR1", literal("striker2")),
                                        ("CHILD_VAR3", literal("striker2")),
                                        // striker1
                                        ("CHILD_VAR2", literal("striker1")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
            ]),
        }
    );
}

#[test]
fn test_inherit_cycle() {
    // One-node cycle
    let mut config = Config {
        applications: map([(
            "app1",
            Application {
                profiles: map([(
                    "child1",
                    Profile {
                        extends: set(["child1"]),
                        variables: map([]),
                    },
                )]),
            },
        )]),
    };
    assert_eq!(
        config
            .resolve_inheritance()
            .expect_err("Expected error for inheritance cycle")
            .to_string(),
        "Inheritance cycle detected: app1/child1 -> app1/child1"
    );

    // Two-node cycle
    let mut config = Config {
        applications: map([(
            "app1",
            Application {
                profiles: map([
                    (
                        "child1",
                        Profile {
                            extends: set(["child2"]),
                            variables: map([]),
                        },
                    ),
                    (
                        "child2",
                        Profile {
                            extends: set(["child1"]),
                            variables: map([]),
                        },
                    ),
                ]),
            },
        )]),
    };
    assert_eq!(
        config
            .resolve_inheritance()
            .expect_err("Expected error for inheritance cycle")
            .to_string(),
        "Inheritance cycle detected: app1/child2 -> app1/child1 -> app1/child2"
    );

    // 3-node cycle
    let mut config = Config {
        applications: map([(
            "app1",
            Application {
                profiles: map([
                    (
                        "child1",
                        Profile {
                            extends: set(["child3"]),
                            variables: map([]),
                        },
                    ),
                    (
                        "child2",
                        Profile {
                            extends: set(["child1"]),
                            variables: map([]),
                        },
                    ),
                    (
                        "child3",
                        Profile {
                            extends: set(["child2"]),
                            variables: map([]),
                        },
                    ),
                ]),
            },
        )]),
    };
    assert_eq!(
        config
            .resolve_inheritance()
            .expect_err("Expected error for inheritance cycle")
            .to_string(),
        "Inheritance cycle detected: app1/child3 -> app1/child2 -> app1/child1 -> app1/child3"
    );
}

#[test]
fn test_inherit_unknown() {
    let mut config = Config {
        applications: map([(
            "app1",
            Application {
                profiles: map([(
                    "child1",
                    Profile {
                        extends: set(["base"]),
                        variables: map([]),
                    },
                )]),
            },
        )]),
    };

    assert_eq!(
        config
            .resolve_inheritance()
            .expect_err("Expected error for unknown path")
            .to_string(),
        "Unknown profile: app1/base"
    );
}
