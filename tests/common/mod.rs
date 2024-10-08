use assert_cmd::Command;
use rstest_reuse::{self, *};
use std::path::{Path, PathBuf};

/// Command to run env-select
pub fn env_select() -> Command {
    let mut command = Command::cargo_bin("es").unwrap();
    command.current_dir(tests_dir());
    command
}

/// Test template to run test with all shells
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

/// Run a script inside the given shell. This will use `es init` to load the
/// correct shell function, then run the script.
///
/// `detect_shell` argument controls whether env-select will guess which shell
/// it's running under (true) or we'll explicitly tell it with -s (false).
pub fn execute_script(
    script: &str,
    shell_kind: &str,
    detect_shell: bool,
) -> Command {
    // Get the function source from `es init`
    let mut es = env_select();
    if detect_shell {
        es.env("SHELL", shell_path(shell_kind));
    } else {
        es.args(["-s", shell_kind]);
    }
    es.arg("init");
    if shell_kind == "zsh" {
        // Completion script doesn't work with zsh because the compdef command
        // isn't present by default. I tried loading it with compinit but that
        // has other issues. This is a shortcut. We aren't trying to test
        // completion anyway, it's just a bonus.
        es.arg("--no-completions");
    }
    let assert = es.assert().success();

    // Inject the function source into the script
    let function_source = String::from_utf8(assert.get_output().stdout.clone())
        .expect("Function output is not valid UTF-8");
    let script = format!("{function_source} {script}");

    let shell = shell_path(shell_kind);
    let mut command = Command::new(&shell);
    command
        // Run from the tests/ directory, so we can use a dedicated config
        .current_dir(tests_dir())
        .env("SHELL", &shell)
        .args(["-c", &script]);
    command
}

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/")
}
