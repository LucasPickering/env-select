//! Test the `set` subcommand

mod common;

use common::*;
use rstest::rstest;
use rstest_reuse::{self, *};

/// Test all shell integrations with a simple `es set` command
#[apply(all_shells)]
fn test_subcommand_set(
    shell_kind: &str,
    #[values(false, true)] detect_shell: bool,
) {
    execute_script(
        "
        es set integration-tests p1

        echo -n \
            $VARIABLE1 \
            $VARIABLE2 \
            $VARIABLE3 \
            $VARIABLE4 \
            $FILE_VARIABLE1 \
            $FILE_VARIABLE2
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
VARIABLE1 = abc
VARIABLE2 = def
VARIABLE3 = ghi
FILE_VARIABLE1 = 123
FILE_VARIABLE2 = 456
abc def ghi 123 456",
    )
    .stderr("");
}
