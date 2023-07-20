use crate::{
    config::{Profile, ValueSource, ValueSourceKind},
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
#[derive(Clone, Debug, Default)]
pub struct Environment(IndexMap<String, ResolvedValue>);

#[derive(Clone, Debug)]
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
    /// running a shell command
    fn resolve_variable(
        &mut self,
        shell: &Shell,
        variable: String,
        ValueSource(value_source): ValueSource,
    ) -> anyhow::Result<()> {
        let raw_value = match value_source.kind {
            ValueSourceKind::Literal { value } => value,
            ValueSourceKind::File { path } => fs::read_to_string(&path)
                .with_context(|| format!("Error loading file {path:?}"))?,

            // Run a program+args locally
            ValueSourceKind::NativeCommand { command } => {
                Shell::execute_native(command)?
            }

            // Run a command locally via the shell
            ValueSourceKind::ShellCommand { command } => {
                shell.execute_shell(&command)?
            }

            // Run a program+args in a kubernetes pod/container
            ValueSourceKind::KubernetesCommand {
                command,
                pod_selector: pod_filter,
                namespace,
                container,
            } => Shell::execute_kubernetes(
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
                self.0.insert(
                    variable,
                    ResolvedValue {
                        value,
                        sensitive: value_source.sensitive,
                    },
                );
            }
        } else {
            self.0.insert(
                variable,
                ResolvedValue {
                    value: raw_value,
                    sensitive: value_source.sensitive,
                },
            );
        }

        Ok(())
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (variable, value) in &self.0 {
            writeln!(f, "{variable} = {value}")?;
        }
        Ok(())
    }
}

impl Display for ResolvedValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Mask sensitive values
        if self.sensitive {
            write!(f, "{}", "*".repeat(self.value.len()))
        } else {
            write!(f, "{}", self.value)
        }
    }
}
