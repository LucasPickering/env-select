use assert_cmd::Command;
use rstest::fixture;
use rstest_reuse::{self, *};
use std::{env, path::PathBuf};

/// Command to run env-select
#[fixture]
pub fn env_select() -> Command {
    Command::cargo_bin("env-select").unwrap()
}

/// Fixture to run test with all shells
#[template]
#[rstest]
pub fn all_shells(#[values("bash", "zsh", "fish")] shell_kind: &str) {}

/// Get the path to the given shell
pub fn shell_path(shell_kind: &str) -> PathBuf {
    let output = Command::new("which").arg(shell_kind).output().unwrap();
    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("Invalid `which` output")
            .trim(),
    )
}

/// Run a script inside the given shell. This will use `env-select init` to
/// load the correct shell function, then
///
/// `detect_shell` argument controls whether env-select will guess which shell
/// it's running under (true) or we'll explicitly tell it with -s (false).
pub fn execute_script(
    script: &str,
    shell_kind: &str,
    detect_shell: bool,
) -> Command {
    // Get the function source from `env-select init`
    let mut es = env_select();
    if detect_shell {
        es.env("SHELL", shell_path(shell_kind));
    } else {
        es.args(["-s", shell_kind]);
    }
    es.arg("init");
    let assert = es.assert().success();

    // Inject the function source into the script
    let function_source = String::from_utf8(assert.get_output().stdout.clone())
        .expect("Function output is not valid UTF-8");
    let script = format!(
        "
        {function_source}
        {script}
        "
    );

    // Add the directory containing env-select to the $PATH
    let env_select_path = PathBuf::from(env!("CARGO_BIN_EXE_env-select"));
    let env_select_dir = env_select_path.parent().unwrap();
    let shell = shell_path(shell_kind);
    let mut command = Command::new(&shell);
    command
        .env("SHELL", &shell)
        .env(
            "PATH",
            format!(
                "{}:{}",
                env::var("PATH").unwrap(),
                env_select_dir.display()
            ),
        )
        .args(["-c", &script]);
    command
}
