mod console;

use crate::console::prompt_value;
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
            println!("export {}={}", args.variable, value);
        }
        None => {
            eprintln!("No values defined for {}", args.variable);
        }
    }

    Ok(())
}
