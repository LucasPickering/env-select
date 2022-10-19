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
    pub variables: HashMap<String, Vec<String>>,
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
            let content = fs::read_to_string(path)?;
            config.merge(toml::from_str(&content)?);
        }

        // Sort each variable's options then remove duplicates
        // We may want to maintain the original order instead of sorting, but
        // let's stick with sorting for now because it feels intuitive and
        // makes deduping easy
        for options in config.variables.values_mut() {
            options.sort();
            options.dedup(); // This requires the vec to be sorted!
        }

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
            let path = dir.join(FILE_NAME);
            if path.exists() {
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
        for (variable, options) in other.variables.into_iter() {
            self.variables
                .entry(variable)
                // TODO remove clone?
                .and_modify(|e| e.extend(options.iter().cloned()))
                .or_insert(options);
        }
    }
}
