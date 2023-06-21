mod config;
mod console;
mod shell;

use crate::{
    config::{Config, Value},
    console::{prompt_application, prompt_variable},
    shell::Shell,
};
use anyhow::anyhow;
use atty::Stream;
use clap::{Parser, Subcommand};
use log::{error, LevelFilter};
use std::{env, process::ExitCode};

/// A utility to select between predefined values or sets of environment
/// variables.
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,

    /// Increase output verbosity, for debugging
    // TODO support multiple levels of verbosity
    #[clap(short, long)]
    verbose: bool,
}

#[derive(Clone, Debug, Subcommand)]
enum Commands {
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
        .filter_level(if args.verbose {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        })
        .init();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Print the error. Most of the time this is a user error, but this
            // will also handle system errors or application bugs. The user
            // should pass -v to get a stack trace for debugging.
            if args.verbose {
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
    let shell = Shell::detect()?;

    match &args.command {
        Commands::Set {
            select_key,
            profile,
        } => {
            match select_key {
                Some(select_key) => {
                    // Figure out what commands we want to feed to the shell,
                    // based on input
                    let export_command = get_export_command(
                        select_key,
                        profile.as_deref(),
                        &config,
                        shell,
                    )?;

                    // Print the command(s) so the user can copy/pipe it to
                    // their shell
                    print_export_command(shell, &export_command);
                    Ok(())
                }
                None => Err(config.get_suggestion_error(
                    "No variable or application provided. {}",
                )),
            }
        }
        Commands::Show => {
            println!("{}", toml::to_string(&config)?);
            Ok(())
        }
    }
}

/// Prompt the user to select a value/profile for the variable/application they
/// gave, then use that to calculate the command(s) we want to feed to the shell
/// to set the desired environment variables. This is basically the whole
/// program.
fn get_export_command(
    select_key: &str,
    profile_name: Option<&str>,
    config: &Config,
    shell: Shell,
) -> anyhow::Result<String> {
    // Check for single variable first
    if let Some(var_options) = config.variables.get(select_key) {
        let value = match profile_name {
            // This is kinda weird, why are you using env-select to just pass
            // a single value? You could just run the shell command directly...
            // Regardless, we might as well support this instead of ignoring it
            // or throwing an error
            Some(value) => Value::Literal(value.into()),
            // The standard use case - prompt the user to pick a value
            None => prompt_variable(select_key, var_options)?.clone(),
        };

        Ok(shell.export_variable(select_key, &value))
    }
    // Check for applications next
    else if let Some(application) = config.applications.get(select_key) {
        let profile = match profile_name {
            // User passed a profile name as an arg - look for a profile of
            // that name
            Some(profile_name) => {
                application.profiles.get(profile_name).ok_or_else(|| {
                    anyhow!(
                        "No profile with the name {}, options are: {}",
                        profile_name,
                        application
                            .profiles
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?
            }
            // Show a prompt to ask the user which varset to use
            None => prompt_application(application)?,
        };

        Ok(shell.export_profile(profile))
    } else {
        // Didn't match anything :(
        Err(config.get_suggestion_error(
            "No known variable or application by the name `{}`. {}",
        ))
    }
}

/// Print the export command, and if apppropriate, tell the user about a sick
/// pro tip.
fn print_export_command(shell: Shell, export_command: &str) {
    // If stdout isn't redirected, then tell the user how to do that
    // for OPTIMAL PERFORMANCE GAINS
    if atty::is(Stream::Stdout) {
        // Normally we don't want to print anything to stdout except for the
        // commands, but in this case we know that stdout isn't being piped
        // anywhere, so it's safe to send regular output there. That way, if
        // the user happens to be piping stderr somewhere, they still see this
        // warning.
        println!(
            "  HINT: Pipe command output to `{}` to apply values automatically",
            shell.source_command()
        );
        println!(
            "  E.g. `{} VARIABLE | {}`",
            env::args().next().unwrap_or("env-select".into()),
            shell.source_command()
        );
        println!("Run the command(s) below to apply variables changes:");
        println!();
    }

    println!("{}", export_command);
}
