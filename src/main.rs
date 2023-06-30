mod config;
mod console;
mod export;
mod shell;

use crate::{config::Config, export::Exporter, shell::Shell};
use anyhow::anyhow;
use clap::{Parser, Subcommand};
use log::{error, LevelFilter};
use std::{iter, path::PathBuf, process::ExitCode};

const BINARY_NAME: &str = env!("CARGO_BIN_NAME");

/// A utility to select between predefined values or sets of environment
/// variables.
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,

    /// Path to the shell binary in use. If omitted, it will be auto-detected
    /// from the $SHELL variable. Supported shell types: bash, zsh, fish
    #[clap(short, long)]
    shell_path: Option<PathBuf>,

    /// Increase output verbosity, for debugging. Supports up to -vv
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

    /// Modify environment from a variable or application name
    Set {
        /// The name of the variable or application to select a value for
        select_key: Option<String>,

        /// Profile to select. If not specified, an interactive prompt will be
        /// shown to select between possible options.
        ///
        /// This also supports literal values for single variables.
        profile: Option<String>,
    },

    /// Show current configuration, with all available variables and
    /// applications
    Show,
}

fn main() -> ExitCode {
    let args = Args::parse();
    env_logger::Builder::new()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .filter_level(match args.verbose {
            0 => LevelFilter::Info,
            1 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
        .init();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Print the error. Most of the time this is a user error, but this
            // will also handle system errors or application bugs. The user
            // should pass -v to get a stack trace for debugging.
            if args.verbose > 0 {
                error!("{error}\n{}", error.backtrace());
            } else {
                error!("{error}");
            }
            ExitCode::FAILURE
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
    let shell = match &args.shell_path {
        Some(shell_path) => Shell::from_path(shell_path)?,
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
        Commands::Set {
            select_key,
            profile,
        } => match select_key {
            Some(select_key) => {
                let exporter = Exporter::new(config, shell);
                exporter.print_export_commands(select_key, profile.as_deref())
            }
            None => Err(config
                .get_suggestion_error("No variable or application provided.")),
        },
        Commands::Show => {
            println!("Shell: {shell}");
            println!();
            println!("{}", toml::to_string(&config)?);
            Ok(())
        }
    }
}
