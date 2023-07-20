mod config;
mod console;
mod environment;
mod error;
mod shell;

use crate::{
    config::{Config, Name},
    console::prompt_options,
    environment::Environment,
    error::ExitCodeError,
    shell::{Shell, ShellKind},
};
use anyhow::{anyhow, bail};
use atty::Stream;
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
        selection: Selection,

        /// Command to execute, as <PROGRAM> [ARGUMENTS]...
        #[arg(required = true, last = true)]
        command: Vec<String>,
    },

    /// Modify shell environment via a configured variable/application
    Set {
        #[command(flatten)]
        selection: Selection,
    },

    /// Show current configuration, with all available variables and
    /// applications
    Show,
}

/// Arguments required for any subcommand that allows applcation/profile
/// selection.
#[derive(Clone, Debug, clap::Args)]
struct Selection {
    /// The name of the application to select a profile for. If omitted, an
    /// interactive prompt will be shown to select between possible options
    application: Option<Name>,

    /// Profile to select. If omitted, an interactive prompt will be shown to
    /// select between possible options.
    profile: Option<Name>,
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
    let verbose = args.verbose > 0;

    fn run(args: Args) -> anyhow::Result<()> {
        let executor = Executor::new(args)?;
        executor.run()
    }

    match run(args) {
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

/// Singleton container for executing commands
struct Executor {
    args: Args,
    config: Config,
    shell: Shell,
}

impl Executor {
    fn new(args: Args) -> anyhow::Result<Self> {
        // This handler will put the terminal cursor back if the user ctrl-c's
        // during the interactive dialogue
        // https://github.com/mitsuhiko/dialoguer/issues/77
        ctrlc::set_handler(move || {
            let term = dialoguer::console::Term::stdout();
            let _ = term.show_cursor();
        })?;

        let config = Config::load()?;
        let shell = match args.shell {
            Some(kind) => Shell::from_kind(kind),
            None => Shell::detect()?,
        };

        Ok(Self {
            args,
            config,
            shell,
        })
    }

    /// Fallible main function
    fn run(&self) -> anyhow::Result<()> {
        match &self.args.command {
            Commands::Init => self.shell.print_init_script(),
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
                selection:
                    Selection {
                        application,
                        profile,
                    },
                command,
            } => self.run_command(
                command,
                application.as_ref(),
                profile.as_ref(),
            ),
            Commands::Set {
                selection:
                    Selection {
                        application,
                        profile,
                    },
            } => self
                .print_export_commands(application.as_ref(), profile.as_ref()),
            Commands::Show => {
                println!("{}", toml::to_string(&self.config)?);
                Ok(())
            }
        }
    }

    /// Build an [Environment] by loading an option for the given select key
    /// (variable or application). If a default value/profile is given, load
    /// that. If not, ask the user for select a value/profile via a TUI
    /// prompt.
    fn load_environment(
        &self,
        application_name: Option<&Name>,
        profile_name: Option<&Name>,
    ) -> anyhow::Result<Environment> {
        let application =
            prompt_options(&self.config.applications, application_name)?;
        let profile = prompt_options(&application.profiles, profile_name)?;
        Environment::from_profile(&self.shell, profile)
    }

    /// Run a command in a sub-environment
    fn run_command(
        &self,
        command: &[String],
        application_name: Option<&Name>,
        profile_name: Option<&Name>,
    ) -> anyhow::Result<()> {
        let environment =
            self.load_environment(application_name, profile_name)?;
        let [program, arguments @ ..] = command else {
            // This *shouldn't* be possible because we marked the
            // argument as required, so clap should
            // reject an empty command
            bail!("Empty command")
        };

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

    /// Print the export command, and if appropriate, tell the user about a
    /// sick pro tip.
    fn print_export_commands(
        &self,
        application_name: Option<&Name>,
        profile_name: Option<&Name>,
    ) -> anyhow::Result<()> {
        let environment =
            self.load_environment(application_name, profile_name)?;

        self.shell.print_export(&environment);

        // Tell the user what we exported, on stderr so it doesn't interfere
        // with shell piping.
        if atty::isnt(Stream::Stdout) {
            eprintln!("The following variables will be set:");
            eprint!("{environment}");
        }

        console::print_installation_hint()?;
        Ok(())
    }
}
