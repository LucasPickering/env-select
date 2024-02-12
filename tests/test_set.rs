//! Test the `set` subcommand

mod common;

use common::*;
use rstest::rstest;
use rstest_reuse::{self, *};

/// Test all shell integrations with a simple `es set` command
#[apply(all_shells)]
fn test_set_subcommand(
    shell_kind: &str,
    #[values(false, true)] detect_shell: bool,
) {
    execute_script(
        "
        es set test p1
        echo -n $VAR1 $VAR2 $VAR3 $FILE_VAR1 $FILE_VAR2
        ",
        shell_kind,
        detect_shell,
    )
    .assert()
    .success()
    .stdout(
        "pre setup 1
pre setup 2
post setup 1 abc
post setup 2 abc
The following variables will be set:
VAR1 = abc
VAR2 = def
FILE_VAR1 = 123
abc def 123",
    )
    .stderr("");
}
