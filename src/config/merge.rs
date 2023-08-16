use super::Config;
use crate::config::ProfileReference;
use indexmap::{map::Entry, IndexMap};
use log::warn;
use std::{hash::Hash, path::Path};

impl Config {
    /// Merge another config into this one. This is similar to inheritance, but
    /// simpler. This is used only for merging multiple config files together.
    /// We only merge down to the profile level. If the same profile is defined
    /// in both files, our version will be used and the other will be thrown
    /// out.
    pub(super) fn merge(&mut self, other: Self, other_path: &Path) {
        // Merge applications together. It would've been nice to use the trait
        // pattern like Qualify and Inherit, but it turns out it complicates
        // this a lot because of the need for context passing.
        merge_map(
            &mut self.applications,
            other.applications,
            |application_name, self_application, other_application| {
                // Merge profiles together
                merge_map(
                    &mut self_application.profiles,
                    other_application.profiles,
                    // If two profiles conflict, just print a warning
                    |profile_name, _, _| {
                        // ProfileReference gives us consistent formatting
                        let reference: ProfileReference =
                            (application_name.clone(), profile_name).into();
                        warn!(
                            "Duplicate definition for profile `{reference}`. \
                            Definition from `{}` will not be used.",
                            other_path.display()
                        )
                    },
                )
            },
        );
    }
}

/// Merge two maps together. If there's a conflict between two entries, call the
/// given function, which can trigger a recursive sub-merge if it wants.
fn merge_map<K: Clone + Eq + Hash, V>(
    alpha: &mut IndexMap<K, V>,
    beta: IndexMap<K, V>,
    on_conflict: impl Fn(K, &mut V, V),
) {
    for (k, other_v) in beta {
        match alpha.entry(k) {
            Entry::Occupied(mut entry) => {
                // The clone is ugly, but it skirts lifetime issues. Turns out
                // to be necessary anyway since we need an owned Name to make
                // a ProfileReference above
                on_conflict(entry.key().clone(), entry.get_mut(), other_v)
            }
            Entry::Vacant(entry) => {
                entry.insert(other_v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::Profile,
        test_util::{config, literal, map, set},
    };
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    #[test]
    fn test_merge_config() {
        let alpha_profile = Profile {
            extends: set([]),
            pre_export: vec![],
            post_export: vec![],
            variables: map([("VARIABLE1", literal("alpha"))]),
        };
        let beta_profile = Profile {
            extends: set([]),
            pre_export: vec![],
            post_export: vec![],
            variables: map([("VARIABLE1", literal("beta"))]),
        };
        let mut alpha_config = config(vec![(
            "app1",
            vec![
                ("no_conflict", alpha_profile.clone()),
                ("conflict", alpha_profile.clone()),
            ],
        )]);
        let beta_config = config(vec![
            ("app1", vec![("conflict", beta_profile.clone())]),
            // Different app - no conflict
            ("app2", vec![("no_conflict", beta_profile.clone())]),
        ]);
        alpha_config.merge(beta_config, &PathBuf::new());
        assert_eq!(
            alpha_config,
            config(vec![
                (
                    "app1",
                    vec![
                        ("no_conflict", alpha_profile.clone()),
                        ("conflict", alpha_profile)
                    ],
                ),
                ("app2", vec![("no_conflict", beta_profile)])
            ])
        );
    }
}
