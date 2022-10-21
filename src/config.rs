use anyhow::Context;
use log::{debug, error, trace};
use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

const FILE_NAME: &str = ".env-select.toml";

/// Add configuration, as loaded from one or more config files
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    /// A set of possible values for individual variables. Each variable maps
    /// to zero or more possible values, and the user can select from this
    /// list for each variable *independently* of the other variables.
    #[serde(default, rename = "vars")]
    pub variables: HashMap<String, Vec<String>>,

    /// A set of possible *multi-variable mappings*. Think of this as a table:
    /// The columns are varsets, the rows are definitions of varsets. Each cell
    /// contains a 1:1 mapping of *multiple* variables, each one with a
    /// singular value. For example:
    ///
    /// |servers              |vegetables                      |
    /// |---------------------|--------------------------------|
    /// |VAR1="dev",VAR2="dev"|VARA="tomato",VARB="potato"     |
    /// |VAR1="prd",VAR2="prd"|VARA="eggplant",VARB="groundhog"|
    ///
    /// The user selects which *column* they care about as a command argument,
    /// and they select which *row* within that column via the interactive
    /// prompt.
    ///
    /// Note that for a single column, each cell *does not necessarily contain
    /// the same set of variables*. Some may be omitted!
    #[serde(default, rename = "varsets")]
    pub variable_sets: HashMap<String, Vec<HashMap<String, String>>>,
}

impl Config {
    /// Load config from the current directory and all parents. Any config
    /// file in any directory in the hierarchy will be loaded and merged into
    /// the config, with lower files take precedence.
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Config::default();

        // Iterate *backwards*, so that we go top->bottom in the dir tree.
        // Lower files should have higher precedence.
        for path in Self::get_all_files()?.iter().rev() {
            debug!("Loading config from file {path:?}");
            let content = fs::read_to_string(path)
                .with_context(|| format!("Error reading file {path:?}"))?;
            match toml::from_str(&content) {
                Ok(parsed) => {
                    trace!("Loaded from file {path:?}: {parsed:?}");
                    config.merge(parsed);
                }
                Err(error) => {
                    error!("{path:?} will be ignored due to error: {error}")
                }
            }
        }

        // Sort each variable's options then remove duplicates
        // We may want to maintain the original order instead of sorting, but
        // let's stick with sorting for now because it feels intuitive and
        // makes deduping easy
        for options in config.variables.values_mut() {
            options.sort();
            options.dedup(); // This requires the vec to be sorted!
        }

        debug!("Loaded config: {config:#?}");
        Ok(config)
    }

    /// Starting at the current directory, walk *up* the tree and collect the
    /// list of all config files.
    fn get_all_files() -> anyhow::Result<Vec<PathBuf>> {
        let cwd = env::current_dir()?;

        let mut config_files: Vec<PathBuf> = Vec::new();
        let mut search_dir: Option<&Path> = Some(&cwd);
        // Walk *up* the tree until we've hit the root
        while let Some(dir) = search_dir {
            trace!("Scanning for config file in {dir:?}");
            let path = dir.join(FILE_NAME);
            if path.exists() {
                trace!("Found config file at {path:?}");
                config_files.push(path);
            }
            search_dir = dir.parent();
        }

        Ok(config_files)
    }

    /// Merge another config file into this one. The two maps of variables will
    /// be merged together, and the value lists for any duplicate variables will
    /// be appended together
    fn merge(&mut self, other: Self) {
        // For each variable, append options onto our existing list
        for (variable, options) in other.variables.into_iter() {
            self.variables.entry(variable).or_default().extend(options);
        }

        // Each each varset key, append options onto our existing list
        for (name, options) in other.variable_sets.into_iter() {
            self.variable_sets.entry(name).or_default().extend(options);
        }
    }
}
