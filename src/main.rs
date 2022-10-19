mod console;
mod shell;

use crate::{console::prompt_value, shell::Shell};
use atty::Stream;
use clap::Parser;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde::Deserialize;
use std::collections::HashMap;

const FILE_NAME: &str = ".env-select.toml";

/// TODO
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// TODO
    variable: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Config {
    variables: HashMap<String, Vec<String>>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    // TODO walk up the directory tree to find more files
    let config: Config =
        Figment::new().merge(Toml::file(FILE_NAME)).extract()?;

    match config.variables.get(&args.variable) {
        Some(values) => {
            // Show a prompt to ask the user which value to use
            let value = prompt_value(&args.variable, values)?;

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
                eprintln!(
                    "  E.g. `env-select VARIABLE | {}`",
                    shell.source_command()
                );
            }

            println!("{}", export_command);
        }
        None => {
            eprintln!("No values defined for {}", args.variable);
        }
    }

    Ok(())
}
