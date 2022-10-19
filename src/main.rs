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
    let config: Config =
        Figment::new().merge(Toml::file(FILE_NAME)).extract()?;

    println!("Values for {}:", args.variable);
    dbg!(config.variables.get(&args.variable));

    Ok(())
}
