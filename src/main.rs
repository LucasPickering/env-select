mod config;
mod console;
mod error;
mod export;
mod shell;

use crate::{
    config::Config,
    error::ExitCodeError,
    export::Exporter,
    shell::{Shell, ShellKind},
};
use anyhow::{anyhow, bail};
use clap::{Parser, Subcommand};
use log::{error, info, LevelFilter};
use std::{
    iter,
    process::{Command, ExitCode},
};

const BINARY_NAME: &str = env!("CARGO_BIN_NAME");

/// A utility to select between predefined values or sets of environment
/// variables.
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,

    /// Type of the shell binary in use. If omitted, it will be auto-detected
    /// from the $SHELL variable.
    #[clap(short, long)]
    shell: Option<ShellKind>,

    /// Increase output verbosity, for debugging. Supports up to -vvv
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Clone, Debug, Subcommand)]
enum Commands {
    /// Configure the shell environment for env-select. Intended to be piped
    /// to `source` as part of your shell startup.
    Init,

    /// Test the given env-select command to see if it's a `set` command. This
    /// is only useful for the wrapping shell functions; it tells them if they
    /// should attempt to source the output of the command. The given command
    /// is *not* executed, just parsed by clap. Return exit code 0 if it's a
    /// `set` command, 1 otherwise.
    #[command(hide = true)]
    Test {
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Run a command in an augmented environment, via a configured
    /// variable/application
    Run {
        #[command(flatten)]
        selection_args: SelectionArgs,

        /// Command to execute, as <PROGRAM> [ARGUMENTS]...
        #[arg(required = true, last = true)]
        command: Vec<String>,
    },

    /// Modify shell environment via a configured variable/application
    Set {
        #[command(flatten)]
        selection_args: SelectionArgs,
    },

    /// Show current configuration, with all available variables and
    /// applications
    Show,
}

/// Arguments required for any subcommand that allows applcation/profile
/// selection.
#[derive(Clone, Debug, clap::Args)]
struct SelectionArgs {
    /// The name of the application to select a profile for
    // TODO make this optional and allow selecting application interactively
    application: String,

    /// Profile to select. If omitted, an interactive prompt will be shown to
    /// select between possible options.
    profile: Option<String>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    env_logger::Builder::new()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .filter_level(match args.verbose {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
        .init();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // If the error includes an exit code, use it
            match error.downcast::<ExitCodeError>() {
                // If we're propagating the exit code, we don't want to print
                // the error. This is for `env-select run`, which means
                // stdout/stderr have been forwarded and we don't want to tack
                // on any more logging.
                Ok(error) => error.into(),
                // For most errors, print it. Most of the time this is a user
                // error, but this will also handle system errors or application
                // bugs. The user should pass -v to get a stack trace for
                // debugging.
                // https://docs.rs/anyhow/1.0.71/anyhow/struct.Error.html#display-representations
                Err(error) => {
                    if args.verbose > 0 {
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

/// Fallible main function. If this errors out, it can be handled by `main`.
fn run(args: &Args) -> anyhow::Result<()> {
    // This handler will put the terminal cursor back if the user ctrl-c's
    // during the interactive dialogue
    // https://github.com/mitsuhiko/dialoguer/issues/77
    ctrlc::set_handler(move || {
        let term = dialoguer::console::Term::stdout();
        let _ = term.show_cursor();
    })?;

    let config = Config::load()?;
    let shell = match args.shell {
        Some(kind) => Shell::from_kind(kind)?,
        None => Shell::detect()?,
    };

    match &args.command {
        Commands::Init => shell.print_init_script(),
        Commands::Test { command } => {
            // Attempt to parse the given command, and check if it's a `set`
            match Args::try_parse_from(
                iter::once(BINARY_NAME)
                    .chain(command.iter().map(String::as_str)),
            ) {
                Ok(Args {
                    command: Commands::Set { .. },
                    ..
                }) => Ok(()),
                Ok(_) => Err(anyhow!("Not a `set` command: {command:?}")),
                Err(_) => Err(anyhow!("Invalid command: {command:?}")),
            }
        }
        Commands::Run {
            selection_args:
                SelectionArgs {
                    application: select_key,
                    profile,
                },
            command,
        } => {
            let [program, arguments @ ..] = command.as_slice() else {
                // This *shouldn't* be possible because we marked the argument
                // as required, so clap should reject an empty command
                bail!("Empty command")
            };
            let exporter = Exporter::new(config, shell);
            let environment =
                exporter.load_environment(select_key, profile.as_deref())?;

            info!("Executing {program:?} {arguments:?} with extra environment {environment}");
            let status = Command::new(program)
                .args(arguments)
                .envs(environment.iter_unmasked())
                .status()?;

            if status.success() {
                Ok(())
            } else {
                // Forward exit code to user
                Err(ExitCodeError::from(&status).into())
            }
        }
        Commands::Set {
            selection_args:
                SelectionArgs {
                    application: select_key,
                    profile,
                },
        } => {
            let exporter = Exporter::new(config, shell);
            exporter.print_export_commands(select_key, profile.as_deref())
        }
        Commands::Show => {
            println!("Shell: {}", shell.kind);
            println!();
            println!("{}", toml::to_string(&config)?);
            Ok(())
        }
    }
}
