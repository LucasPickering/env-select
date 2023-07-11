//! Utilitied related to profile inheritance resolution

use crate::config::{Config, Profile, ProfileReference};
use anyhow::anyhow;
use indexmap::IndexSet;

impl Config {
    /// Resolve `extends` field for all profiles
    pub(super) fn resolve_inheritance(&mut self) -> anyhow::Result<()> {
        // Step 1 - Make sure application is set for all profile references
        self.qualify_profile_references();

        // Profiles we've *started* (and possibly finished) resolving
        let mut visited: IndexSet<ProfileReference> = IndexSet::new();
        // Profiles we've *finished* resolving
        let mut resolved: IndexSet<ProfileReference> = IndexSet::new();

        // Step 2 - resolve dependency tree
        todo!();

        // Step 3 - merge configs together
        todo!();

        Ok(())
    }

    /// Fully qualify all profile references. It'd be nice to have a different
    /// type to enforce the reference is resolved, but it's not worth drilling
    /// that all the way down the tree so we'll just do runtime checks later to
    /// be safe
    fn qualify_profile_references(&mut self) {
        // First, go through and
        for (application_name, application) in &mut self.applications {
            for profile in application.profiles.values_mut() {
                // We have to drain the set and rebuild it, since the hashes
                // will change
                profile.extends = profile
                    .extends
                    .drain(..)
                    .map(|mut reference| {
                        if reference.application.is_none() {
                            reference.application =
                                Some(application_name.clone());
                        }
                        reference
                    })
                    .collect();
            }
        }
    }

    /// Get a profile by reference. This should only be called *after*
    /// qualifying all profile references
    fn get_profile(
        &self,
        reference: &ProfileReference,
    ) -> anyhow::Result<Option<&Profile>> {
        let application = reference.application.as_ref().ok_or_else(|| {
            anyhow!(
                "Unqualified profile reference {:?} during inheritance \
                resolution. This is a bug!",
                reference
            )
        })?;
        Ok(self.applications.get(application).and_then(|application| {
            application.profiles.get(&reference.profile)
        }))
    }
}
