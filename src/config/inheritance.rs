//! Utilitied related to profile inheritance resolution

use crate::config::{Config, DisplayKeys, Name, Profile, ProfileReference};
use anyhow::{anyhow, bail};
use indexmap::{IndexMap, IndexSet};
use log::trace;
use std::{collections::HashMap, fmt::Display, hash::Hash};

impl Config {
    /// Resolve inheritance for all profiles. Each profile will have its parents
    /// (as specified in its `extends` field) merged into it, recursively.
    pub(super) fn resolve_inheritance(&mut self) -> anyhow::Result<()> {
        let mut resolver = InheritanceResolver::from_config(self);
        resolver.resolve_all()
    }
}

struct InheritanceResolver<'a> {
    profiles: HashMap<QualifiedReference, &'a mut Profile>,
    unresolved: IndexMap<QualifiedReference, IndexSet<QualifiedReference>>,
}

impl<'a> InheritanceResolver<'a> {
    fn from_config(config: &'a mut Config) -> Self {
        let mut profiles = HashMap::new();
        let mut unresolved = IndexMap::new();

        // Flatten profiles into a map, keyed by their path. For each profile,
        // we'll also track a list of parents that haven't been resolved+merged
        // in yet
        for (application_name, application) in &mut config.applications {
            for (profile_name, profile) in &mut application.profiles {
                let reference =
                    QualifiedReference::new(application_name, profile_name);
                // Qualify relative references using the parent application
                let parents = QualifiedReference::qualify_all(
                    application_name,
                    profile.extends.iter(),
                );

                // Any profile with parents is deemed unresolved
                profiles.insert(reference.clone(), profile);
                if !parents.is_empty() {
                    unresolved.insert(reference, parents);
                }
            }
        }

        trace!(
            "Detected {} profiles needing inheritance resolution: {}",
            unresolved.len(),
            unresolved.display_keys()
        );
        Self {
            profiles,
            unresolved,
        }
    }

    /// Resolve inheritance for all profiles
    fn resolve_all(&mut self) -> anyhow::Result<()> {
        // Resolve each profile. A profile has been resolved when its `parents`
        // list is empty, so keep going until they're all done
        while let Some((reference, parents)) = self.unresolved.pop() {
            self.resolve_profile(reference, parents, &mut IndexSet::new())?;
        }
        Ok(())
    }

    /// Resolve inheritance for a single profile, recursively. This will also
    /// resolve its parents, and their parents, and so on.
    fn resolve_profile(
        &mut self,
        reference: QualifiedReference,
        parents: IndexSet<QualifiedReference>,
        visited: &mut IndexSet<QualifiedReference>,
    ) -> anyhow::Result<()> {
        trace!("Resolving inheritance for profile {reference}");
        visited.insert(reference.clone());

        for parent in parents {
            trace!("Resolving parent {reference} -> {parent}");

            // Check for cycles
            if visited.contains(&parent) {
                bail!("Inheritance cycle detected: {}", display_cycle(visited));
            }

            // Check if parent needs to be resolved. If parent is an unknown
            // path, we'll skip over here and fail down below
            if let Some(grandparents) = self.unresolved.remove(&parent) {
                // Parent is unresolved - resolve it now
                self.resolve_profile(
                    parent.clone(),
                    grandparents,
                    // When we branch, we have to clone the `visited` list so
                    // it remains linear. This doesn't seem like a good
                    // solution, and yet it works...
                    &mut visited.clone(),
                )?;
            } else {
                trace!("{parent} is resolved");
            }

            // We know parent is resolved now, merge in their values
            Self::apply_inheritance(
                (*self
                    .profiles
                    .get(&parent)
                    .ok_or_else(|| anyhow!("Unknown profile: {}", parent))?)
                .clone(),
                self.profiles
                    .get_mut(&reference)
                    .ok_or_else(|| anyhow!("Unknown profile: {}", reference))?,
            );
        }
        Ok(())
    }

    /// Merge a parent into a child, i.e. any data in the parent but *not* the
    /// child will be added to the child
    fn apply_inheritance(parent: Profile, child: &mut Profile) {
        // TODO can we use Merge for this? Right now it's parent-prefential,
        // but we may change that
        for (variable, parent_value) in parent.variables {
            // Only insert the parent value if it isn't already in the child
            child.variables.entry(variable).or_insert(parent_value);
        }
    }
}

/// A [ProfileReference] that has been qualified with its application name,
/// such that it is globally unique
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct QualifiedReference {
    application: Name,
    profile: Name,
}

impl QualifiedReference {
    fn new(application: &Name, profile: &Name) -> Self {
        Self {
            application: application.clone(),
            profile: profile.clone(),
        }
    }

    /// Qualify all references for a given application. Any relative
    /// references will be qualified with the application name
    fn qualify_all<'a>(
        parent_application: &'a Name,
        references: impl IntoIterator<Item = &'a ProfileReference>,
    ) -> IndexSet<Self> {
        references
            .into_iter()
            .map(|reference| QualifiedReference {
                application: reference
                    .application
                    .as_ref()
                    .unwrap_or(parent_application)
                    .clone(),
                profile: reference.profile.clone(),
            })
            .collect()
    }
}

impl Display for QualifiedReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.application, self.profile)
    }
}

/// Pretty print a cycle chain
fn display_cycle<T: Display>(nodes: &IndexSet<T>) -> String {
    let mut output = String::new();
    for node in nodes {
        output.push_str(&node.to_string());
        output.push_str(" -> ");
    }
    // Duplicate the first node at the end, to show the cycle
    output.push_str(&nodes[0].to_string());
    output
}
