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
        es set TEST_VARIABLE success
        es set integration-tests p1
        echo -n \
            $TEST_VARIABLE \
            $PROFILE_VARIABLE_1 \
            $PROFILE_VARIABLE_2 \
            $PROFILE_VARIABLE_3 \
            $PROFILE_VARIABLE_4
        ",
        shell_kind,
        detect_shell,
    )
    .assert()
    .success()
    .stdout("success abc def ghi jkl");
}
