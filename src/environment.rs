use crate::{
    config::{Profile, ValueSource, ValueSourceKind},
    shell::Shell,
};
use anyhow::{anyhow, Context};

use futures::future;
use indexmap::IndexMap;
use log::info;
use smol::fs;
use std::fmt::{Display, Formatter};

/// Container of VARIABLE=value mappings. This handles resolving value sources
/// into values, including processing multi-value outputs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Environment(IndexMap<String, ResolvedValue>);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedValue {
    value: String,
    sensitive: bool,
}

impl Environment {
    /// Create a new environment from a mapping of variable=value. This will
    /// resolve the value(s) if necessary.
    pub fn from_profile(
        shell: &Shell,
        profile: &Profile,
    ) -> anyhow::Result<Self> {
        let mut environment = Self::default();

        // Resolve all values in parallel
        let resolved: Vec<(&str, &ValueSource, String)> = smol::block_on(
            future::try_join_all(profile.variables.iter().map(
                |(variable, value_source)| async move {
                    info!("Resolving {variable} = {value_source}");
                    let value =
                        Self::resolve_value(shell, value_source).await?;
                    Ok::<_, anyhow::Error>((
                        variable.as_str(),
                        value_source,
                        value,
                    ))
                },
            )),
        )?;

        for (variable, value_source, value) in resolved {
            environment.apply_variable(
                variable.to_owned(),
                value_source,
                value,
            )?;
        }
        Ok(environment)
    }

    /// Get an iterator over unmasked `(variable, value)` pairs that can be
    /// exported to the shell
    pub fn iter_unmasked(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(variable, value)| (variable.as_str(), value.value.as_str()))
    }

    /// Update this environment with a resolved value string. If it's a
    /// multi-variable mapping, parse it and insert all sub-variables.
    fn apply_variable(
        &mut self,
        variable: String,
        ValueSource(value_source): &ValueSource,
        raw_value: String,
    ) -> anyhow::Result<()> {
        if value_source.multiple.enabled() {
            // If we're expecting a multi-value mapping, parse that now. We'll
            // throw away the variable name from the config and use the ones in
            // the mapping
            let mapping = dotenv_parser::parse_dotenv(&raw_value)
                .map_err(|error| anyhow!(error))
                .with_context(|| {
                    format!(
                        "Error parsing multi-variable mapping for field {}",
                        variable
                    )
                })?;

            for (variable, value) in mapping {
                // Check if the user specified only certain variables to include
                if value_source.multiple.includes(&variable) {
                    self.insert(variable, value, value_source.sensitive);
                }
            }
        } else {
            self.insert(variable, raw_value, value_source.sensitive);
        }

        Ok(())
    }

    /// Calculate the raw value from a value source. For multi-value sources,
    /// the mapping string will be returned.
    async fn resolve_value(
        shell: &Shell,
        ValueSource(value_source): &ValueSource,
    ) -> anyhow::Result<String> {
        // Resolve the string value, which could be treated as one value or a
        // mapping of multiple down below
        match &value_source.kind {
            ValueSourceKind::Literal { value } => Ok(value.clone()),
            ValueSourceKind::File { path } => fs::read_to_string(path)
                .await
                .with_context(|| format!("Error loading file {path:?}")),

            // Run a command locally via the shell
            ValueSourceKind::Command { command, cwd } => {
                let mut executable = shell.executable(command);
                // If cwd is given, use that. Otherwise inherit from the user
                if let Some(cwd) = cwd {
                    executable.current_dir(cwd);
                }
                executable.check_output().await
            }
        }
    }

    /// Insert a variable=value mapping into the environment
    fn insert(&mut self, variable: String, value: String, sensitive: bool) {
        // If the variable is PATH, add to it instead of overidding
        let value = if Shell::is_path_variable(&variable) {
            Shell::prepend_path(value)
        } else {
            value
        };
        self.0.insert(variable, ResolvedValue { value, sensitive });
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Regular:
        // VARIABLE1 = "value", VARIABLE2 = "value"
        // Alternate/pretty:
        // VARIABLE1 = "value"
        // VARIABLE2 = "value"

        for (i, (variable, value)) in self.0.iter().enumerate() {
            // Write separator for subsequent entries
            if i > 0 {
                if f.alternate() {
                    writeln!(f)?;
                } else {
                    write!(f, ", ")?;
                }
            }

            write!(f, "{variable} = {value}")?;
        }

        Ok(())
    }
}

impl Display for ResolvedValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Mask sensitive values
        if self.sensitive {
            write!(f, "<REDACTED>")
        } else {
            write!(f, "{}", self.value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ValueSourceInner,
        shell::ShellKind,
        test_util::{all_shells, command, file, literal, map},
    };
    use rstest::rstest;
    use rstest_reuse::apply;
    use std::env;

    #[test]
    fn test_resolve_literal() {
        assert_eq!(
            environment(map([
                ("VARIABLE1", literal("test")),
                ("VARIABLE2", literal("test").sensitive()),
            ]))
            .unwrap(),
            Environment(map([
                ("VARIABLE1", resolved_value("test")),
                (
                    "VARIABLE2",
                    ResolvedValue {
                        value: "test".into(),
                        sensitive: true
                    }
                ),
            ]))
        );
    }

    #[apply(all_shells)]
    fn test_resolve_command(shell_kind: ShellKind) {
        let current_dir = env::current_dir().unwrap();
        let temp_dir = env::temp_dir().canonicalize().unwrap();
        let temp_dir = temp_dir.to_string_lossy();
        assert_eq!(
            environment_shell(
                shell_kind,
                map([
                    ("VARIABLE1", command("echo test")),
                    ("VARIABLE2", command("pwd")),
                    ("VARIABLE3", command("pwd").cwd(&temp_dir)),
                ])
            )
            .unwrap(),
            Environment(map([
                ("VARIABLE1", resolved_value("test")),
                ("VARIABLE2", resolved_value(current_dir.to_string_lossy())),
                ("VARIABLE3", resolved_value(temp_dir)),
            ]))
        );
    }

    #[test]
    fn test_resolve_file() {
        let path = env::temp_dir().join("test_file");
        std::fs::write(&path, "test").unwrap();
        assert_eq!(
            environment(map([("VARIABLE1", file(&path))])).unwrap(),
            Environment(map([("VARIABLE1", resolved_value("test"))]))
        );
    }

    #[test]
    fn test_resolve_multiple() {
        // Load all from the mapping
        assert_eq!(
            environment(map([(
                "multi", // This is thrown away
                literal("VARIABLE1=test1\nVARIABLE2=test2").multiple()
            )]))
            .unwrap(),
            Environment(map([
                ("VARIABLE1", resolved_value("test1")),
                ("VARIABLE2", resolved_value("test2")),
            ]))
        );

        // Load only a filtered set
        assert_eq!(
            environment(map([(
                "multi", // This is thrown away
                literal("VARIABLE1=test1\nVARIABLE2=test2")
                    .multiple_filtered(&["VARIABLE1"])
            )]))
            .unwrap(),
            Environment(map([("VARIABLE1", resolved_value("test1"))]))
        );

        assert_eq!(
            environment(map([("multi", literal("=test1").multiple())]))
                .unwrap_err()
                .to_string(),
            "Error parsing multi-variable mapping for field multi".to_string()
        );
    }

    #[apply(all_shells)]
    fn test_path_variable(shell_kind: ShellKind) {
        let base_path = env::var("PATH").unwrap();
        let expected = Environment(map([(
            "PATH",
            resolved_value(format!("~/.bin:{base_path}")),
        )]));

        // Set PATH as a single variable
        assert_eq!(
            environment_shell(shell_kind, map([("PATH", literal("~/.bin"))]),)
                .unwrap(),
            expected
        );

        // Set PATH as a multi-variable mapping
        assert_eq!(
            environment_shell(
                shell_kind,
                map([(
                    "_",
                    ValueSource(ValueSourceInner {
                        kind: ValueSourceKind::Literal {
                            value: "PATH=~/.bin".into()
                        },
                        multiple: true.into(),
                        sensitive: false,
                    })
                )]),
            )
            .unwrap(),
            expected
        );
    }

    /// Helper for building an environment with a default shell kind
    fn environment(
        variables: IndexMap<String, ValueSource>,
    ) -> anyhow::Result<Environment> {
        environment_shell(ShellKind::Bash, variables)
    }

    /// Helper for building an environment with a specific shell kind
    fn environment_shell(
        shell_kind: ShellKind,
        variables: IndexMap<String, ValueSource>,
    ) -> anyhow::Result<Environment> {
        Environment::from_profile(
            &shell_kind.into(),
            &Profile {
                variables,
                ..Default::default()
            },
        )
    }

    /// Helper for building a resolved value
    fn resolved_value<T: Into<String>>(value: T) -> ResolvedValue {
        ResolvedValue {
            value: value.into(),
            sensitive: false,
        }
    }
}
