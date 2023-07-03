mod common;

use assert_cmd::Command;
use common::*;
use rstest::rstest;
use rstest_reuse::{self, *};

#[template]
#[rstest]
fn all_shells(#[values("bash", "zsh", "fish")] shell_kind: &str) {}

/// Test all shell integrations with a simple `es set` command
#[apply(all_shells)]
fn test_set(
    mut env_select: Command,
    shell_kind: &str,
    #[values(false, true)] infer: bool,
) {
    if infer {
        env_select.args(["-s", shell_kind]);
    } else {
        env_select.env("SHELL", shell_path(shell_kind));
    }
    env_select.arg("init");
    let assert = env_select.assert().success();

    // Pipe the function source to the shell
    let function_source = String::from_utf8(assert.get_output().stdout.clone())
        .expect("Invalid function output");
    // This script should run in all shells
    let script = format!(
        "
        {function_source}
        es set TEST_VARIABLE success
        es set integration-tests p1
        echo -n $TEST_VARIABLE $PROFILE_VARIABLE_1 $PROFILE_VARIABLE_2
        "
    );
    shell(shell_kind)
        .args(["-c", &script])
        .assert()
        .success()
        .stdout("success abc def");
}
