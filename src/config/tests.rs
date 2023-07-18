//! Home to config *parsing* tests, as well as common test utils..
//! Module-specific tests are in their own files.

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
[applications.server.profiles.prd.variables]
SERVICE1 = "prd"
SERVICE2 = "also-prd"
multiple = {type = "literal", value = "MULTI1=multi1\nMULTI2=multi2", multiple = true}

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
            multiple: false,
        })
    }
}

// Builder-like functions to make it easy to create value sources
impl ValueSource {
    fn sensitive(mut self, sensitive: bool) -> Self {
        self.0.sensitive = sensitive;
        self
    }

    fn multiple(mut self, multiple: bool) -> Self {
        self.0.multiple = multiple;
        self
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
                                    (
                                        "multiple",
                                        literal("MULTI1=multi1\nMULTI2=multi2")
                                            .multiple(true),
                                    ),
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

/// Test generic fields on ValueSource
#[test]
fn test_parse_value_source() {
    assert_tokens(
        &literal("abc").multiple(true).sensitive(true).0,
        &[
            Token::Map { len: None },
            Token::Str("type"),
            Token::Str("literal"),
            Token::Str("value"),
            Token::Str("abc"),
            Token::Str("multiple"),
            Token::Bool(true),
            Token::Str("sensitive"),
            Token::Bool(true),
            Token::MapEnd,
        ],
    );
}

#[test]
fn test_parse_literal() {
    // Flat syntax
    assert_de_tokens(&literal("abc"), &[Token::Str("abc")]);
    assert_de_tokens(&literal("true"), &[Token::Bool(true)]);
    assert_de_tokens(&literal("-16"), &[Token::I8(-16)]);
    assert_de_tokens(&literal("-16"), &[Token::I16(-16)]);
    assert_de_tokens(&literal("-16"), &[Token::I32(-16)]);
    assert_de_tokens(&literal("-16"), &[Token::I64(-16)]);
    assert_de_tokens(&literal("16"), &[Token::U8(16)]);
    assert_de_tokens(&literal("16"), &[Token::U16(16)]);
    assert_de_tokens(&literal("16"), &[Token::U32(16)]);
    assert_de_tokens(&literal("16"), &[Token::U64(16)]);
    assert_de_tokens(&literal("420.69000244140625"), &[Token::F32(420.69)]);
    assert_de_tokens(&literal("420.69"), &[Token::F64(420.69)]);

    // Map syntax
    assert_tokens(
        &literal("abc").0.kind,
        &[
            Token::Struct {
                name: "ValueSourceKind",
                len: 2,
            },
            Token::Str("type"),
            Token::Str("literal"),
            Token::Str("value"),
            Token::Str("abc"),
            Token::StructEnd,
        ],
    );
}

#[test]
fn test_parse_native_command() {
    assert_tokens(
        &native("echo", ["test"]).0.kind,
        &[
            Token::Struct {
                name: "ValueSourceKind",
                len: 2,
            },
            Token::Str("type"),
            Token::Str("command"),
            Token::Str("command"),
            Token::Seq { len: Some(2) },
            Token::Str("echo"),
            Token::Str("test"),
            Token::SeqEnd,
            Token::StructEnd,
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
    assert_tokens(
        &shell("echo test").0.kind,
        &[
            Token::Struct {
                name: "ValueSourceKind",
                len: 2,
            },
            Token::Str("type"),
            Token::Str("shell"),
            Token::Str("command"),
            Token::Str("echo test"),
            Token::StructEnd,
        ],
    );
}

#[test]
fn test_parse_kubernetes() {
    assert_tokens(
        &ValueSourceKind::KubernetesCommand {
            command: NativeCommand {
                program: "printenv".to_owned(),
                arguments: vec!["DB_PASSWORD".to_owned()],
            },
            pod_selector: "app=api".to_owned(),
            namespace: Some("development".to_owned()),
            container: Some("main".to_owned()),
        },
        &[
            Token::Struct {
                name: "ValueSourceKind",
                len: 5,
            },
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
            Token::StructEnd,
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
            `literal`, `file`, `command`, `shell`, `kubernetes`",
    )
}
