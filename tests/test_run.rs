//! Test the `run` subcommand

mod common;

use common::*;
use rstest::rstest;
use rstest_reuse::{self, *};

/// Test `es run` executes the command within a subshell, and the variables
/// don't leak outside that subprocess
#[apply(all_shells)]
fn test_run_subcommand(shell_kind: &str) {
    execute_script(
        "
        es run test p1 -- printenv VAR1 VAR2 VAR3 FILE_VAR1 FILE_VAR2
        echo Empty: $VAR1
        ",
        shell_kind,
        true,
    )
    .assert()
    .success()
    .stdout(
        "pre setup 1
pre setup 2
post setup 1 abc
post setup 2 abc
abc
def
123
post teardown 2 abc
post teardown 1 abc
pre teardown 2
pre teardown 1
Empty:
",
    )
    .stderr("");
}

/// Test `es run` forwards quotes and other shell features in the command
/// properly
#[apply(all_shells)]
fn test_run_escaping(shell_kind: &str) {
    execute_script(
        "es run test empty -- echo -n '$NOT_EXPANDED' '\"$(hello!!)\"'",
        shell_kind,
        true,
    )
    .assert()
    .success()
    .stdout("$NOT_EXPANDED \"$(hello!!)\"")
    .stderr("");
}
