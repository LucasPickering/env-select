mod config;
mod console;
mod shell;

use crate::{
    config::Config,
    console::{prompt_variable, prompt_variable_set},
    shell::Shell,
};
use atty::Stream;
use clap::Parser;
use log::{error, LevelFilter};

/// TODO
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The name of the variable or variable set to select a value for
    select_key: String,

    /// Increase output verbosity, for debugging
    // TODO support multiple levels of verbosity
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
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
    let config = Config::load()?;
    let shell = Shell::detect()?;

    // Figure out what commands we want to feed to the shell, based on input
    let export_command =
        match get_export_command(&args.select_key, &config, shell)? {
            Some(export_command) => export_command,
            None => {
                error!(
                    "No known variables or variable sets by the name {}",
                    &args.select_key
                );
                return Ok(());
            }
        };

    // Print the command(s) so the user can copy/pipe it to their shell
    print_export_command(shell, &export_command);

    Ok(())
}

/// Prompt the user to select a value/value set for the variable/variable set
/// they gave, then use that to calculate the command(s) we want to feed to the
/// shell to set the desired environment variables. This is basically the whole
/// program.
///
/// Returns `Ok(None))` if the select key doesn't match any known variables or
/// variable sets.
fn get_export_command(
    select_key: &str,
    config: &Config,
    shell: Shell,
) -> anyhow::Result<Option<String>> {
    if let Some(var_options) = config.variables.get(select_key) {
        // Show a prompt to ask the user which variable options to use
        let value = prompt_variable(select_key, var_options)?;

        Ok(Some(shell.export_variable(select_key, value)))
    } else if let Some(varset_options) = config.variable_sets.get(select_key) {
        // Show a prompt to ask the user which varset to use
        let varset = prompt_variable_set(varset_options)?;

        Ok(Some(shell.export_variables(varset)))
    } else {
        Ok(None)
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
        println!("  E.g. `es VARIABLE | {}`", shell.source_command());
        println!("Run the command(s) below to apply variables changes:");
        println!();
    }

    println!("{}", export_command);
}
