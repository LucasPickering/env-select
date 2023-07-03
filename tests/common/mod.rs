use assert_cmd::Command;
use rstest::fixture;
use std::{env, path::PathBuf};

/// Command to run env-select
#[fixture]
pub fn env_select() -> Command {
    Command::cargo_bin("env-select").unwrap()
}

/// Get the path to the given shell
pub fn shell_path(shell_type: &str) -> PathBuf {
    let output = Command::new("which").arg(shell_type).output().unwrap();
    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("Invalid `which` output")
            .trim(),
    )
}

/// Command to the given shell. The directory containing env-select will be
/// added to the PATH.
pub fn shell(shell_type: &str) -> Command {
    // Add the directory containing env-select to the $PATH
    let env_select_path = PathBuf::from(env!("CARGO_BIN_EXE_env-select"));
    let env_select_dir = env_select_path.parent().unwrap();
    let shell = shell_path(shell_type);
    let mut command = Command::new(&shell);
    command.env("SHELL", &shell).env(
        "PATH",
        format!("{}:{}", env::var("PATH").unwrap(), env_select_dir.display()),
    );
    command
}
