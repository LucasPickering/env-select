mod config;
mod console;
mod shell;

use crate::{config::Config, console::prompt_options, shell::Shell};
use atty::Stream;
use clap::Parser;
use log::LevelFilter;

/// TODO
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// TODO
    variable: String,

    // TODO support multiple levels of verbosity
    #[clap(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    env_logger::Builder::new()
        .format_timestamp(None)
        .filter_level(if args.verbose {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        })
        .init();
    let config = Config::load()?;

    match config.variables.get(&args.variable) {
        Some(values) => {
            // Show a prompt to ask the user which value to use
            let value = match prompt_options(&args.variable, values)? {
                Some(value) => value,
                // None means user quit the prompt, so just give up
                None => return Ok(()),
            };

            // Generate a shell command
            let shell = Shell::detect()?;
            let export_command = shell.export_variable(&args.variable, &value);

            // If stdout isn't redirected, then tell the user how to do that
            // for OPTIMAL PERFORMANCE GAINS
            if atty::is(Stream::Stdout) {
                eprintln!("Run the command below to apply variables changes");
                eprintln!(
                    "  HINT: Pipe command output to `{}` to apply variables automatically",
                    shell.source_command()
                );
                eprintln!("  E.g. `es VARIABLE | {}`", shell.source_command());
            }

            println!("{}", export_command);
        }
        None => {
            eprintln!("No values defined for {}", args.variable);
        }
    }

    Ok(())
}
