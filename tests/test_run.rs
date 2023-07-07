//! Test the `run` subcommand

mod common;

use common::*;
use rstest::rstest;
use rstest_reuse::{self, *};

/// Test that `es run` exports variables only for the single command
#[apply(all_shells)]
fn test_subcommand_run(shell_kind: &str) {
    // We need ||true because printenv fails when given unknown vars
    let printenv_command = "printenv \
        TEST_VARIABLE \
        PROFILE_VARIABLE_1 \
        PROFILE_VARIABLE_2 \
        PROFILE_VARIABLE_3 \
        PROFILE_VARIABLE_4 || true
    ";
    execute_script(
        &format!(
            "
            es run TEST_VARIABLE success -- {printenv_command}
            echo
            es run integration-tests p1 -- {printenv_command}
            "
        ),
        shell_kind,
        true,
    )
    .assert()
    .success()
    .stdout("success\n\nabc\ndef\nghi\njkl\n");
}
