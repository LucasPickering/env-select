use crate::{
    config::ShellCommand,
    environment::Environment,
    execute::{Executable, IntoExecutable},
};
use anyhow::anyhow;
use clap::ValueEnum;
use derive_more::Display;
use log::{debug, info};
use std::{
    env,
    ffi::OsStr,
    fmt::{Debug, Display, Formatter, Write},
    path::PathBuf,
};

/// https://en.wikipedia.org/wiki/PATH_(variable)
const PATH_VARIABLE: &str = "PATH";

/// In each wrapper, this key will be replaced by the path to env-select
const BINARY_REPLACEMENT_KEY: &str = "ENV_SELECT_BINARY";
const BASH_WRAPPER: &str = include_str!("../shells/es.sh");
const ZSH_WRAPPER: &str = include_str!("../shells/es.sh");
const FISH_WRAPPER: &str = include_str!("../shells/es.fish");

/// A pointer to a specific type of shell
#[derive(Clone, Debug)]
pub struct Shell {
    pub kind: ShellKind,
    /// Path to the shell. Only populated if the shell is detected from $SHELL,
    /// because then it's easily available. We keep this as String instead of
    /// PathBuf because it's loaded from a string anyway, and it reduces the
    /// amount of boilerplate we need.
    pub path: Option<String>,
}

/// A supported kind of shell. The display implementation here defines the
/// binary name that we'll use to invoke it
#[derive(Copy, Clone, Debug, Display, ValueEnum)]
pub enum ShellKind {
    #[display(fmt = "bash")]
    Bash,
    #[display(fmt = "zsh")]
    Zsh,
    #[display(fmt = "fish")]
    Fish,
}

impl Shell {
    /// Detect the current shell from the $SHELL variable.
    pub fn detect() -> anyhow::Result<Self> {
        let path = env::var("SHELL")?;
        debug!("Detected shell path from $SHELL: {path}");
        let path_buf = PathBuf::from(&path);
        let shell_name = path_buf
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(anyhow!("Failed to read shell type from path: {path}"))?;
        let kind = ShellKind::from_str(shell_name, true)
            .map_err(|message| anyhow!("{}", message))?;
        info!("Detected shell type: {kind}");
        Ok(Self {
            path: Some(path),
            kind,
        })
    }

    /// Create a shell of the given kind. Since no path is given, we'll just
    /// hope it's in PATH if we ever need to execute it.
    pub fn from_kind(kind: ShellKind) -> Self {
        Self { path: None, kind }
    }

    /// Is the given variable the PATH variable? PATH gets special functionality
    /// to prepend instead of replacing
    pub fn is_path_variable(variable: &str) -> bool {
        variable == PATH_VARIABLE
    }

    /// Add a new directory to the *beginning* of the PATH variable. This will
    /// give the new directory priority over all others. It stands to reason
    /// that if a user is adding a directory specifically for one environment,
    /// they would want it to override any potential duplicates.
    pub fn prepend_path(new_path: String) -> String {
        env::var(PATH_VARIABLE)
            .map(|full_path| format!("{new_path}:{full_path}"))
            .unwrap_or(new_path)
    }

    /// Get a valid shell script that will initialize the `es` wrapper as well
    /// as whatever other initialization is needed. The script should be piped
    /// to `source`.
    pub fn init_script(&self) -> anyhow::Result<String> {
        let wrapper_template = match self.kind {
            ShellKind::Bash => BASH_WRAPPER,
            ShellKind::Zsh => ZSH_WRAPPER,
            ShellKind::Fish => FISH_WRAPPER,
        };

        // Inject the path of the current binary into the script. This prevents
        // any need to modify PATH
        Ok(wrapper_template.replace(
            BINARY_REPLACEMENT_KEY,
            &env::current_exe()?.display().to_string(),
        ))
    }

    /// Get the shell command(s) that will configure the environment to a
    /// particular set of key=value pairs for this shell type. This command
    /// can later be piped to the source command to apply it.
    pub fn export(&self, environment: &Environment) -> String {
        let mut output = String::new();
        for (variable, value) in environment.iter_unmasked() {
            // Escape single quotes to prevent injection vulnerabilities
            let variable = self.escape(variable);
            let value = self.escape(value);

            // Generate a shell command to export the variable
            match self.kind {
                ShellKind::Bash | ShellKind::Zsh => {
                    writeln!(output, "export {variable}={value}")
                        .expect("string writing is infallible");
                }
                ShellKind::Fish => {
                    writeln!(output, "set -gx {variable} {value}")
                        .expect("string writing is infallible");
                }
            }
        }
        output
    }

    /// Get an [Executable] command to run in this shell, from a shell command
    pub fn executable(&self, command: &ShellCommand) -> Executable {
        // Use the full shell path if we have it. Otherwise, just pass
        // the shell name and hope it's in PATH
        let shell_program =
            self.path.clone().unwrap_or_else(|| self.kind.to_string());
        (shell_program, ["-c", command]).executable()
    }

    /// Get an [Executable] command to run in this shell, from a program name +
    /// args. This will wrap each element in double quotes to preserve inner
    /// quotes/parentheses/etc.
    ///
    /// This powers the arg de-parsing for `es run`. The goal is to execute the
    /// command exactly as the shell would if `es run` weren't involved. E.g.:
    ///
    /// ```
    /// es run app prof -- echo '$TEST'
    /// ```
    ///
    /// should execute the same exact shell command as
    ///
    /// ```
    /// echo '$TEST'
    /// ```
    ///
    /// This means `$TEST` should *not* be expanded, because it's in single
    /// quotes. The quotes will be dropped by the shell before `es run` is
    /// invoked, so we need to add quotes back.
    pub fn executable_from_slice(&self, command_args: &[String]) -> Executable {
        // All supported shells currently treat single/double quotes the same so
        // we can use unified logic
        let command = command_args
            .iter()
            .map(|s| self.escape(s))
            .collect::<Vec<_>>()
            .join(" ");
        self.executable(&command.into())
    }

    /// Wrap the given string in single quotes, escaping inner single quotes.
    /// This makes it safe to pass as a shell argument without possibility of
    /// injection.
    fn escape(&self, value: &str) -> String {
        let escaped = match self.kind {
            // Bash and zsh don't support escaping ' with just a backslash.
            // You have to terminate the current string, add a single quote,
            // then open a new string
            // https://stackoverflow.com/a/1250279/1907353
            ShellKind::Bash | ShellKind::Zsh => value.replace('\'', "'\\''"),
            ShellKind::Fish => value.replace('\'', "\\'"),
        };
        format!("'{escaped}'")
    }
}

impl Display for Shell {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{} (from $SHELL)", path),
            None => write!(f, "{} (from $PATH)", self.kind),
        }
    }
}

impl From<ShellKind> for Shell {
    fn from(kind: ShellKind) -> Self {
        Self::from_kind(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Profile,
        test_util::{all_shells, literal, map},
    };
    use assert_cmd::Command;
    use rstest::rstest;
    use rstest_reuse::apply;

    /// Bash and Zsh use the same export format so we can test them together
    #[rstest]
    fn test_bash_zsh_export(
        #[values(ShellKind::Bash, ShellKind::Zsh)] shell_kind: ShellKind,
    ) {
        let shell = Shell::from_kind(shell_kind);
        let environment = environment(&shell);
        assert_eq!(
            shell.export(&environment).as_str(),
            "\
export 'SIMPLE'='simple'
export 'ESCAPED'\\''oops'\\'''=''\\''; echo bobby tables '\\'''
"
        );
    }

    /// Fish has its own variable syntax so it needs to be tested separately
    #[test]
    fn test_fish_export() {
        let shell = Shell::from_kind(ShellKind::Fish);
        let environment = environment(&shell);
        assert_eq!(
            shell.export(&environment).as_str(),
            "\
set -gx 'SIMPLE' 'simple'
set -gx 'ESCAPED\\'oops\\'' '\\'; echo bobby tables \\''
"
        );
    }

    fn environment(shell: &Shell) -> Environment {
        Environment::from_profile(
            shell,
            &Profile {
                variables: map([
                    ("SIMPLE", literal("simple")),
                    ("ESCAPED'oops'", literal("'; echo bobby tables '")),
                ]),
                ..Default::default()
            },
        )
        .unwrap()
    }

    /// A pseudo-integration test to make sure quotes and other scary characters
    /// are escaped properly, so they *aren't* handled by the shell.
    #[apply(all_shells)]
    #[case(&[], "")]
    #[case(&["echo", "-n", "$TEST"], "$TEST")]
    #[case(&["echo", "-n", "double \"", "single '"], "double \" single '")]
    #[case(&["echo", "-n", "$(echo scary!)"], "$(echo scary!)")]
    fn test_executable_from_args(
        shell_kind: ShellKind,
        #[case] input: &[&str],
        #[case] expected: &'static str,
    ) {
        let shell = Shell::from_kind(shell_kind);
        let command: Vec<String> =
            input.iter().copied().map(str::to_owned).collect();
        let executable = shell.executable_from_slice(&command);
        Command::new(executable.program())
            .args(executable.arguments())
            .assert()
            .success()
            .stdout(expected)
            .stderr("");
    }
}
