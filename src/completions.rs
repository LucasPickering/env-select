use crate::config::{Config, Name};
use clap_complete::CompletionCandidate;
use std::ffi::OsStr;

/// Provide completions for application names
pub fn complete_application(current: &OsStr) -> Vec<CompletionCandidate> {
    let Ok(config) = Config::load() else {
        return Vec::new();
    };

    get_candidates(config.applications.keys().map(Name::as_str), current)
}

/// Provide completions for profile names
pub fn complete_profile(current: &OsStr) -> Vec<CompletionCandidate> {
    let Ok(config) = Config::load() else {
        return Vec::new();
    };

    // Suggest all profiles for all applications. Ideally we could grab the
    // prior argument to tell us what application we're in, but I'm not sure if
    // clap exposes that at all
    get_candidates(
        config
            .applications
            .values()
            .flat_map(|application| application.profiles.keys())
            .map(Name::as_str),
        current,
    )
}

fn get_candidates<'a>(
    iter: impl Iterator<Item = &'a str>,
    current: &OsStr,
) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    // Only include IDs prefixed by the input we've gotten so far
    iter.filter(|value| value.starts_with(current))
        .map(CompletionCandidate::new)
        .collect()
}
