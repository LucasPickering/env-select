mod console;
mod shell;

use crate::{console::prompt_value, shell::Shell};
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
            let shell = Shell::detect()?;
            let export_command = shell.export_variable(&args.variable, &value);

            // TODO only print this if stdout isn't already redirect
            eprintln!(
                "Pipe command output to `{}` to apply it. E.g. `env-select VARIABLE | source`",
                shell.source_command()
            );
            println!("{}", export_command);
        }
        None => {
            eprintln!("No values defined for {}", args.variable);
        }
    }

    Ok(())
}
