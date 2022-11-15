//! Generate shell completions at build time

use clap::{Command, CommandFactory};
use clap_complete::{
    generate_to,
    shells::{Bash, Fish, Zsh},
    Generator,
};
use std::{env, ffi::OsString, io::Error};

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let out_dir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(out_dir) => out_dir,
    };

    let mut cmd = Args::command();
    // The list here should match the Shell type in the crate
    generate_completion(Bash, &mut cmd, &out_dir)?;
    generate_completion(Zsh, &mut cmd, &out_dir)?;
    generate_completion(Fish, &mut cmd, &out_dir)?;

    Ok(())
}

fn generate_completion(
    shell: impl Generator,
    cmd: &mut Command,
    out_dir: &OsString,
) -> Result<(), Error> {
    let path = generate_to(shell, cmd, "es", out_dir)?;
    println!("cargo:warning=Generated completion to {:?}", path);
    Ok(())
}
