use crate::{
    config::{Profile, ValueSource, ValueSourceKind},
    execute::execute_kubernetes,
    shell::Shell,
};
use anyhow::{anyhow, Context};

use indexmap::IndexMap;
use std::{
    fmt::{Display, Formatter},
    fs,
};

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
        for (variable, value) in &profile.variables {
            environment.resolve_variable(
                shell,
                variable.into(),
                value.clone(),
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

    /// Get a string for a Value. This may involve external communication, e.g.
    /// running a shell command. Resolved value(s) will be inserted into the
    /// environment.
    fn resolve_variable(
        &mut self,
        shell: &Shell,
        variable: String,
        ValueSource(value_source): ValueSource,
    ) -> anyhow::Result<()> {
        // Resolve the string value, which could be treated as one value or a
        // mapping of multiple down below
        let raw_value = match value_source.kind {
            ValueSourceKind::Literal { value } => value,
            ValueSourceKind::File { path } => fs::read_to_string(&path)
                .with_context(|| format!("Error loading file {path:?}"))?,

            // Run a command locally via the shell
            ValueSourceKind::Command { command } => {
                shell.executable(&command).check_output()?
            }

            // Run a program+args in a kubernetes pod/container
            ValueSourceKind::KubernetesCommand {
                command,
                pod_selector: pod_filter,
                namespace,
                container,
            } => execute_kubernetes(
                &command,
                &pod_filter,
                namespace.as_deref(),
                container.as_deref(),
            )?,
        };

        if value_source.multiple {
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
                self.insert(variable, value, value_source.sensitive);
            }
        } else {
            self.insert(variable, raw_value, value_source.sensitive);
        }

        Ok(())
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
        test_util::{all_shells, literal, map},
    };
    use rstest::rstest;
    use rstest_reuse::apply;
    use std::env;

    #[apply(all_shells)]
    fn test_path_variable(shell_kind: ShellKind) {
        let base_path = "/bin:/usr/bin";
        let expected = Environment(map([(
            "PATH",
            ResolvedValue {
                value: format!("~/.bin:{base_path}"),
                sensitive: false,
            },
        )]));
        // Override PATH so we get consistent results
        env::set_var("PATH", base_path);

        // Set PATH as a single variable
        assert_eq!(
            Environment::from_profile(
                &shell_kind.into(),
                &Profile {
                    variables: map([("PATH", literal("~/.bin"))]),
                    ..Default::default()
                }
            )
            .unwrap(),
            expected
        );

        // Set PATH as a multi-variable mapping
        assert_eq!(
            Environment::from_profile(
                &shell_kind.into(),
                &Profile {
                    variables: map([(
                        "_",
                        ValueSource(ValueSourceInner {
                            kind: ValueSourceKind::Literal {
                                value: "PATH=~/.bin".into()
                            },
                            multiple: true,
                            sensitive: false,
                        })
                    )]),
                    ..Default::default()
                }
            )
            .unwrap(),
            expected
        );
    }
}
