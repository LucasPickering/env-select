mod config;
mod console;
mod environment;
mod error;
mod shell;

use crate::{
    config::{Config, Name, NativeCommand},
    console::prompt_options,
    environment::Environment,
    error::ExitCodeError,
    shell::{Shell, ShellKind},
};
use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand};
use log::{error, info, LevelFilter};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

/// A utility to select between predefined values or sets of environment
/// variables.
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,

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

#[derive(Clone, Debug, Subcommand)]
enum Commands {
    /// Configure the shell environment for env-select. Intended to be piped
    /// to `source` as part of your shell startup.
    Init,

    /// Run a command in an augmented environment, via a configured
    /// variable/application
    Run {
        #[command(flatten)]
        selection: Selection,

        /// Command to execute, as <PROGRAM> [ARGUMENTS]...
        #[arg(required = true, last = true)]
        command: Vec<String>,

        /// TODO
        #[clap(short, long)]
        run_in_shell: bool,
    },

    /// Modify shell environment via a configured variable/application
    Set {
        #[command(flatten)]
        selection: Selection,
    },

    /// Print configured values. Useful for debugging and completions
    Show(ShowArgs),
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

#[derive(Clone, Debug, clap::Args)]
struct ShowArgs {
    #[command(subcommand)]
    command: ShowSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
enum ShowSubcommand {
    /// Print full resolved configuration, in TOML format
    Config,
    /// Print the name or path to the shell in use
    Shell,
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
        let executor = Executor::new(args.shell)?;
        executor.run(args)
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
    config: Config,
    shell: Shell,
}

impl Executor {
    fn new(shell_kind: Option<ShellKind>) -> anyhow::Result<Self> {
        // This handler will put the terminal cursor back if the user ctrl-c's
        // during the interactive dialogue
        // https://github.com/mitsuhiko/dialoguer/issues/77
        ctrlc::set_handler(move || {
            let term = dialoguer::console::Term::stdout();
            let _ = term.show_cursor();
        })?;

        let config = Config::load()?;
        let shell = match shell_kind {
            Some(kind) => Shell::from_kind(kind),
            None => Shell::detect()?,
        };

        Ok(Self { config, shell })
    }

    /// Fallible main function
    fn run(self, args: Args) -> anyhow::Result<()> {
        match args.command {
            Commands::Init => self.print_init_script(),
            Commands::Run {
                selection:
                    Selection {
                        application,
                        profile,
                    },
                command,
                run_in_shell,
            } => self.run_command(
                command,
                application.as_ref(),
                profile.as_ref(),
                run_in_shell,
            ),
            Commands::Set {
                selection:
                    Selection {
                        application,
                        profile,
                    },
            } => self.write_export_commands(
                application.as_ref(),
                profile.as_ref(),
                &args.source_file.ok_or_else(|| {
                    anyhow!(
                        "--source-file argument required for subcommand `set`"
                    )
                })?,
            ),
            Commands::Show(ShowArgs { command }) => {
                match command {
                    ShowSubcommand::Config => {
                        println!("{}", toml::to_string(&self.config)?)
                    }
                    ShowSubcommand::Shell => println!("{}", self.shell),
                }
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

    /// Print the shell init script, which should be piped to `source`
    fn print_init_script(&self) -> anyhow::Result<()> {
        let script = self
            .shell
            .init_script()
            .context("Error generating shell init script")?;
        println!("{script}");
        console::print_installation_hint()
    }

    /// Run a command in a sub-environment
    fn run_command(
        &self,
        command: Vec<String>,
        application_name: Option<&Name>,
        profile_name: Option<&Name>,
        run_in_shell: bool,
    ) -> anyhow::Result<()> {
        let environment =
            self.load_environment(application_name, profile_name)?;

        // Convert the string command into NativeCommand
        let command: NativeCommand = if run_in_shell {
            // Undo the tokenization from clap
            self.shell.get_shell_command(&command.join(" "))
        } else {
            // This *shouldn't* fail because we marked the argument as required,
            // so clap will reject an empty command
            command.try_into()?
        };

        info!("Executing {command} with extra environment:\n{environment}");
        let status = Command::new(&command.program)
            .args(&command.arguments)
            .envs(environment.iter_unmasked())
            .status()
            .context(command)?;

        if status.success() {
            Ok(())
        } else {
            // Forward exit code to user
            Err(ExitCodeError::from(&status).into())
        }
    }

    /// Write export commands to a file
    fn write_export_commands(
        &self,
        application_name: Option<&Name>,
        profile_name: Option<&Name>,
        source_file: &Path,
    ) -> anyhow::Result<()> {
        let environment =
            self.load_environment(application_name, profile_name)?;

        let source_output = self.shell.export(&environment);
        fs::write(source_file, source_output).with_context(|| {
            format!("Error writing sourceable output to file {source_file:?}")
        })?;

        // Tell the user what we exported
        println!("The following variables will be set:");
        print!("{environment}");

        Ok(())
    }
}
