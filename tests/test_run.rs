//! Test the `run` subcommand

mod common;

use common::*;
use rstest::rstest;
use rstest_reuse::{self, *};

/// Test that `es run` exports variables only for the single command
#[apply(all_shells)]
fn test_subcommand_run_native(shell_kind: &str) {
    // We need ||true because printenv fails when given unknown vars
    let printenv_command = "printenv VARIABLE1 VARIABLE2 VARIABLE3 VARIABLE4 \
        FILE_VARIABLE1 FILE_VARIABLE2";
    execute_script(
        &format!(
            "
            es run integration-tests p1 -- {printenv_command}
            echo Empty: $VARIABLE1
            "
        ),
        shell_kind,
        true,
    )
    .assert()
    .success()
    .stdout("abc\ndef\nghi\njkl\n123\n456\nEmpty:\n");
}

/// Test that `es run --run-in-shell` executes the command within a subshell
#[apply(all_shells)]
fn test_subcommand_run_shell(shell_kind: &str) {
    // We need ||true because printenv fails when given unknown vars
    let echo_command = "echo '$VARIABLE1' '$VARIABLE2' '$VARIABLE3' \
        '$VARIABLE4' '$FILE_VARIABLE1' '$FILE_VARIABLE2'";
    execute_script(
        &format!(
            "
            es run --run-in-shell integration-tests p1 -- {echo_command}
            echo Empty: $VARIABLE1
            "
        ),
        shell_kind,
        true,
    )
    .assert()
    .success()
    .stdout("abc def ghi jkl 123 456\nEmpty:\n");
}
