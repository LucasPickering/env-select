use super::*;
use crate::config::{Application, Config, Profile, ValueSourceKind};
use indexmap::{IndexMap, IndexSet};
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

// TODO add more comprehensive inheritance tests

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Helper for building an IndexMap
fn map<'a, K: Eq + Hash + PartialEq + From<&'a str>, V, const N: usize>(
    items: [(&'a str, V); N],
) -> IndexMap<K, V> {
    items.into_iter().map(|(k, v)| (k.into(), v)).collect()
}

/// Helper for building an IndexMap
fn set<V: Hash + Eq, const N: usize>(items: [V; N]) -> IndexSet<V> {
    IndexSet::from(items)
}

/// Helper for building a ProfileReference
fn profile(application: Option<&str>, profile: &str) -> ProfileReference {
    ProfileReference {
        application: application.map(Name::from),
        profile: profile.into(),
    }
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
                arguments: arguments.into_iter().map(String::from).collect(),
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
                                extends: set([profile(Some("base"), "base")]),
                                variables: map([("USERNAME", literal("user"))]),
                            },
                        ),
                        (
                            "dev",
                            Profile {
                                extends: set([profile(None, "base")]),
                                variables: map([
                                    ("SERVICE1", literal("dev")),
                                    ("SERVICE2", literal("also-dev")),
                                ]),
                            },
                        ),
                        (
                            "prd",
                            Profile {
                                extends: set([profile(None, "base")]),
                                variables: map([
                                    ("SERVICE1", literal("prd")),
                                    ("SERVICE2", literal("also-prd")),
                                ]),
                            },
                        ),
                        (
                            "secret",
                            Profile {
                                extends: set([profile(None, "base")]),
                                variables: map([
                                    ("SERVICE1", literal_sensitive("secret")),
                                    (
                                        "SERVICE2",
                                        native("echo", ["also-secret"], true),
                                    ),
                                    (
                                        "SERVICE3",
                                        shell(
                                            "echo secret_password | base64",
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
    let mut map1: IndexMap<String, _> = map([("a", set([1])), ("b", set([2]))]);
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
        applications: map([
            (
                "app1",
                Application {
                    profiles: map([
                        (
                            "prof1",
                            Profile {
                                extends: set([profile(None, "prof2")]),
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
                                extends: set([profile(None, "prof3")]),
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
                                    extends: set([
                                        profile(None, "prof2"),
                                        profile(None, "prof3")
                                    ]),
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
                                variables: map([("VAR1", literal("val11"),)])
                            },
                        )]),
                    }
                ),
            ]),
        }
    );
}
