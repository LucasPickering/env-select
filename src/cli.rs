use clap::Parser;

/// A utility to select between predefined values or sets of environment
/// variables.
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// The name of the variable or application to select a value for
    pub select_key: String,

    /// Profile to select. If not specified, an interactive prompt will be
    /// shown to select between possible options.
    ///
    /// This also supports literal values for single variables.
    pub profile: Option<String>,

    /// Increase output verbosity, for debugging
    // TODO support multiple levels of verbosity
    #[clap(short, long)]
    pub verbose: bool,
}
