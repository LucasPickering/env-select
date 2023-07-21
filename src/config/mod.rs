mod cereal;
mod inherit;
mod merge;
mod qualify;
#[cfg(test)]
mod tests;

use crate::config::merge::Merge;
use anyhow::{anyhow, bail, Context};
use indexmap::{IndexMap, IndexSet};
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
    str::FromStr,
};

const FILE_NAME: &str = ".env-select.toml";

/// Add configuration, as loaded from one or more config files. We use
/// [indexmap::IndexMap] in here to preserve ordering from the input files.
/// This (hopefully) makes usage more intuitive for the use.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// A set of named applications (as in, a use case, purpose, etc.). An
    /// application typically has one or more variables that control it, and
    /// each variable may multiple values to select between. Each value set
    /// is known as a "profile".
    pub applications: IndexMap<Name, Application>,
}

/// An application is a grouping of profiles. Each profile should be different
/// "versions" of the same "application", e.g. dev vs prd for the same service.
/// Different colors of the same car, so to speak.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Application {
    pub profiles: IndexMap<Name, Profile>,
}

/// An application or profile name. Newtype allows us to apply validation during
/// deserialization.
#[derive(Clone, Debug, Default, Serialize, Hash, Eq, PartialEq)]
pub struct Name(String);

/// A profile is a set of fixed variable mappings, i.e. each variable maps to
/// a singular value.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    /// List of profiles that we'll inherit from. Last has precedence
    pub extends: IndexSet<ProfileReference>,
    /// The meat
    pub variables: IndexMap<String, ValueSource>,
    /// Imperative commands to run *before* resolving an environment
    pub pre_export: Vec<SideEffect>,
    /// Imperative commands to run *after* resolving an environment
    pub post_export: Vec<SideEffect>,
}

/// Pointer to a profile, relative to some "self" profile. (De)serializes as
/// "[application/]profile"
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProfileReference {
    /// Application name. If omitted, the application of the "self" profile is
    /// assumed
    application: Option<Name>,
    /// Profile name
    profile: Name,
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

    /// Source provides a mapping of line-delimited VARIABLE=value settings,
    /// instead of a single vlaue
    #[serde(default)]
    pub multiple: bool,

    /// Value(s) should be masked in display output
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

    /// Load value from a file
    #[serde(rename = "file")]
    File {
        /// File path, relative to the config file that this was defined in
        path: PathBuf,
    },

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

/// A pair of imperative commands to run. The setup command is run during
/// environment setup (either before or after exporting the environment), while
/// the teardown is run during cleanup. The teardown will run in the mirrored
/// position of the setup. E.g. if the setup is run *pre*-export, the teardown
/// will be run *after* clearing the environment.
///
/// Each field is optional to support side effects that don't require teardown
/// (or more rarely, don't require setup). Generally though, you should specify
/// both.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct SideEffect {
    setup: Option<SideEffectCommand>,
    teardown: Option<SideEffectCommand>,
}

/// A single imperative command to run as part of a side effect. Could be either
/// the setup or the teardown.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
#[serde(untagged)]
pub enum SideEffectCommand {
    Native(NativeCommand),
    Shell(String),
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

impl Config {
    /// Load config from the current directory and all parents. Any config
    /// file in any directory in the hierarchy will be loaded and merged into
    /// the config, with lower files take precedence.
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Config::default();

        for path in Self::get_all_files()?.iter() {
            debug!("Loading config from file {path:?}");
            let content = fs::read_to_string(path)
                .with_context(|| format!("Error reading file {path:?}"))?;
            match toml::from_str::<Config>(&content) {
                Ok(mut parsed) => {
                    debug!("Loaded from file {path:?}: {parsed:?}");
                    // Qualify relative paths to be absolute
                    parsed.qualify(path);
                    config.merge(parsed);
                }
                Err(error) => {
                    error!("{path:?} will be ignored due to error: {error}")
                }
            }
        }

        trace!("Loaded config (pre-inheritance): {config:#?}");
        // Resolve all `extends` fields
        config.inherit()?;

        info!("Loaded and resolved config: {config:#?}");
        Ok(config)
    }

    /// Starting at the current directory, walk up the tree and collect the
    /// list of all config files. Return the list of files from
    /// **top-to-bottom**, so that the highest priority file comes last.
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

        // Return top->bottom results
        config_files.reverse();
        Ok(config_files)
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Validate application/profile name. We do a bit of sanity checking here to
// prevent stuff that might be confusing, or collide with env-select features
impl FromStr for Name {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        if name.is_empty() {
            bail!("Invalid name: empty string");
        }

        if name.starts_with(char::is_whitespace)
            || name.ends_with(char::is_whitespace)
        {
            bail!("Invalid name: contains leading/trailing whitespace");
        }

        // Right now we only care about /, but the others might be useful later
        let reserved = &['\\', '/', '*', '?', '!'];
        if name.contains(reserved) {
            bail!(
                "Invalid name: contains one of reserved characters {}",
                reserved.iter().collect::<String>()
            );
        }

        Ok(Self(name.into()))
    }
}

impl ProfileReference {
    /// Is this an absolute reference, i.e. does it include an application name?
    pub fn is_qualified(&self) -> bool {
        self.application.is_some()
    }
}

impl FromStr for ProfileReference {
    type Err = anyhow::Error;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        // Path should be other "profile" or "application/profile". We lean on
        // Name to do the bulk of validation. If there's multiple slashes, the
        // latter will appear in the profile name and get rejected
        match path.split_once('/') {
            None => Ok(ProfileReference {
                application: None,
                profile: path.parse()?,
            }),
            Some((application, profile)) => Ok(ProfileReference {
                application: Some(application.parse()?),
                profile: profile.parse()?,
            }),
        }
    }
}

impl Display for ProfileReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(application) = &self.application {
            write!(f, "{application}/")?;
        }
        write!(f, "{}", self.profile)
    }
}

impl ValueSource {
    /// Build a [ValueSource] from a simple string value. All extra fields
    /// are populated with defaults.
    pub fn from_literal(value: impl ToString) -> Self {
        Self(ValueSourceInner {
            kind: ValueSourceKind::Literal {
                value: value.to_string(),
            },
            multiple: false,
            sensitive: false,
        })
    }
}

impl SideEffect {
    pub fn setup(&self) -> Option<&SideEffectCommand> {
        self.setup.as_ref()
    }

    pub fn teardown(&self) -> Option<&SideEffectCommand> {
        self.teardown.as_ref()
    }
}

impl Display for ValueSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0.kind {
            ValueSourceKind::Literal { value } => write!(f, "\"{value}\""),
            ValueSourceKind::File { path } => write!(f, "{}", path.display()),
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
            write!(f, " \"{argument}\"")?;
        }
        write!(f, "`")?;
        Ok(())
    }
}

// For serialization
impl From<NativeCommand> for Vec<String> {
    fn from(value: NativeCommand) -> Self {
        let mut elements = value.arguments;
        elements.insert(0, value.program); // O(n)! Spooky!
        elements
    }
}

// For deserialization
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

/// Nice little extension trait for IndexMap
pub trait MapExt {
    type Key;
    type Value;

    /// Get a reference to a value by key. If the key isn't in the map, return
    /// an error with a helpful message.
    fn try_get(&self, key: &Self::Key) -> anyhow::Result<&Self::Value>;

    /// Print the keys of this map, comma-delimited
    fn display_keys(&self) -> String {
        self.display_keys_delimited(", ")
    }

    /// Print the keys of this map with the given separator
    fn display_keys_delimited(&self, separator: &str) -> String;
}

impl<K: Display + Eq + Hash + PartialEq, V> MapExt for IndexMap<K, V> {
    type Key = K;
    type Value = V;

    fn try_get(&self, key: &Self::Key) -> anyhow::Result<&Self::Value> {
        self.get(key).ok_or_else(|| {
            anyhow!("Unknown key {}, options are: {}", key, self.display_keys())
        })
    }

    fn display_keys_delimited(&self, separator: &str) -> String {
        self.keys()
            .map(|key| key.to_string())
            .collect::<Vec<_>>()
            .join(separator)
    }
}
