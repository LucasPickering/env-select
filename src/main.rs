mod commands;
mod completions;
mod config;
mod console;
mod environment;
mod error;
mod execute;
#[cfg(test)]
mod test_util;

mod shell;

use crate::{commands::Commands, error::ExitCodeError, shell::ShellKind};
use clap::{CommandFactory, Parser};
use log::{error, LevelFilter};
// https://github.com/la10736/rstest/tree/master/rstest_reuse#cavelets
use clap_complete::CompleteEnv;
#[cfg(test)]
#[allow(clippy::single_component_path_imports)]
use rstest_reuse;
use std::{path::PathBuf, process::ExitCode};

/// A utility to select between predefined values or sets of environment
/// variables.
#[derive(Debug, Parser)]
#[clap(bin_name = "es", author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    global: GlobalArgs,
}

/// Args available to all subcommands
#[derive(Debug, Parser)]
pub struct GlobalArgs {
    /// File that env-select should write sourceable output to. Used only by
    /// commands that intend to modify the parent environment. Shell wrappers
    /// will pass a temporary path here. This needs to be a global arg because
    /// the wrapper doesn't know what subcommand is being run.
    #[clap(long, hide = true)]
    source_file: Option<PathBuf>,

    /// Type of the shell binary in use. If omitted, it will be auto-detected
    /// from the $SHELL variable.
    #[clap(short, long)]
    shell: Option<ShellKind>,

    /// Increase output verbosity, for debugging. Supports up to -vvv
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() -> ExitCode {
    // If COMPLETE var is enabled, process will stop after completions
    CompleteEnv::with_factory(Args::command).complete();
    let args = Args::parse();
    env_logger::Builder::new()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .filter_level(match args.global.verbose {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            3.. => LevelFilter::Trace,
        })
        .init();
    let verbose = args.global.verbose > 0;

    match args.command.execute(args.global) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // If the error includes an exit code, use it
            match error.downcast::<ExitCodeError>() {
                // If we're propagating the exit code, we don't want to print
                // the error. This is for `es run`, which means
                // stdout/stderr have been forwarded and we don't want to tack
                // on any more logging.
                Ok(error) => error.into(),
                // For most errors, print it. Most of the time this is a user
                // error, but this will also handle system errors or application
                // bugs. The user should pass -v to get a stack trace for
                // debugging.
                // https://docs.rs/anyhow/1.0.71/anyhow/struct.Error.html#display-representations
                Err(error) => {
                    if verbose {
                        error!("{error:#}\n{}", error.backtrace());
                    } else {
                        error!("{error:#}");
                    }
                    ExitCode::FAILURE
                }
            }
        }
    }
}
