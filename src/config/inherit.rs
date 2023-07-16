//! Utilitied related to profile inheritance resolution

use crate::config::{Config, DisplayKeys, Profile, ProfileReference};
use anyhow::{anyhow, bail};
use indexmap::{IndexMap, IndexSet};
use log::trace;
use std::{collections::HashMap, fmt::Display};

impl Config {
    /// Resolve inheritance for all profiles. Each profile will have its parents
    /// (as specified in its `extends` field) merged into it, recursively.
    pub(super) fn inherit(&mut self) -> anyhow::Result<()> {
        let mut resolver = InheritanceResolver::from_config(self)?;
        resolver.resolve_all()
    }
}

struct InheritanceResolver<'a> {
    profiles: HashMap<ProfileReference, &'a mut Profile>,
    unresolved: IndexMap<ProfileReference, IndexSet<ProfileReference>>,
}

impl<'a> InheritanceResolver<'a> {
    fn from_config(config: &'a mut Config) -> anyhow::Result<Self> {
        let mut profiles = HashMap::new();
        let mut unresolved = IndexMap::new();

        // Flatten profiles into a map, keyed by their path. For each profile,
        // we'll also track a list of parents that haven't been resolved+merged
        // in yet
        for (application_name, application) in &mut config.applications {
            for (profile_name, profile) in &mut application.profiles {
                let reference = ProfileReference {
                    application: Some(application_name.clone()),
                    profile: profile_name.clone(),
                };

                // Any profile with parents is deemed unresolved
                if !profile.extends.is_empty() {
                    // All references should be made absolute during
                    // qualification, this is just a safety check
                    for parent in &profile.extends {
                        if !parent.is_qualified() {
                            bail!(
                                "Unqualified parent `{}` for profile `{}`",
                                parent,
                                profile_name
                            );
                        }
                    }

                    unresolved
                        .insert(reference.clone(), profile.extends.clone());
                }
                profiles.insert(reference, profile);
            }
        }

        trace!(
            "Detected {} profiles needing inheritance resolution: {}",
            unresolved.len(),
            unresolved.display_keys()
        );
        Ok(Self {
            profiles,
            unresolved,
        })
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
        reference: ProfileReference,
        parents: IndexSet<ProfileReference>,
        visited: &mut IndexSet<ProfileReference>,
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

#[cfg(test)]
mod tests {
    use crate::config::{
        tests::{literal, map, set},
        Application, Config, Profile,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_inherit_single() {
        let mut config = Config {
            applications: map([(
                "app",
                Application {
                    profiles: map([
                        (
                            "base",
                            Profile {
                                extends: set([]),
                                variables: map([
                                    ("VAR1", literal("base")),
                                    ("VAR2", literal("base")),
                                ]),
                            },
                        ),
                        (
                            "child",
                            Profile {
                                extends: set(["app/base"]),
                                variables: map([
                                    ("VAR1", literal("child")),
                                    // VAR2 comes from base
                                    ("VAR3", literal("child")),
                                ]),
                            },
                        ),
                    ]),
                },
            )]),
        };
        config.inherit().expect("Error resolving valid inheritance");
        assert_eq!(
            config,
            Config {
                applications: map([(
                    "app",
                    Application {
                        profiles: map([
                            (
                                "base",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("VAR1", literal("base")),
                                        ("VAR2", literal("base")),
                                    ]),
                                },
                            ),
                            (
                                "child",
                                Profile {
                                    extends: set(["app/base"]),
                                    variables: map([
                                        ("VAR1", literal("child")),
                                        ("VAR3", literal("child")),
                                        ("VAR2", literal("base")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),]),
            }
        );
    }

    #[test]
    fn test_inherit_linear() {
        let mut config = Config {
            applications: map([
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "base",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("VAR1", literal("base")),
                                        ("VAR2", literal("base")),
                                    ]),
                                },
                            ),
                            (
                                "child1",
                                Profile {
                                    extends: set(["app1/base"]),
                                    variables: map([
                                        ("VAR1", literal("child1")),
                                        // VAR2 comes from base
                                        ("VAR3", literal("child1")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "app2",
                    Application {
                        profiles: map([(
                            "child2",
                            Profile {
                                extends: set(["app1/child1"]),
                                variables: map([
                                    ("VAR1", literal("child2")),
                                    // VAR2 comes from base
                                    // VAR3 comes from child1
                                    ("VAR4", literal("child2")),
                                ]),
                            },
                        )]),
                    },
                ),
            ]),
        };
        config.inherit().expect("Error resolving valid inheritance");
        assert_eq!(
            config,
            Config {
                applications: map([
                    (
                        "app1",
                        Application {
                            profiles: map([
                                (
                                    "base",
                                    Profile {
                                        extends: set([]),
                                        variables: map([
                                            ("VAR1", literal("base")),
                                            ("VAR2", literal("base")),
                                        ]),
                                    },
                                ),
                                (
                                    "child1",
                                    Profile {
                                        extends: set(["app1/base"]),
                                        variables: map([
                                            ("VAR1", literal("child1")),
                                            ("VAR3", literal("child1")),
                                            ("VAR2", literal("base")),
                                        ]),
                                    },
                                ),
                            ]),
                        },
                    ),
                    (
                        "app2",
                        Application {
                            profiles: map([(
                                "child2",
                                Profile {
                                    extends: set(["app1/child1"]),
                                    variables: map([
                                        ("VAR1", literal("child2")),
                                        ("VAR4", literal("child2")),
                                        ("VAR3", literal("child1")),
                                        ("VAR2", literal("base")),
                                    ]),
                                },
                            )]),
                        },
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_inherit_nonlinear() {
        let mut config = Config {
            applications: map([
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "base1",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("BASE_VAR1", literal("base1")),
                                        ("BASE_VAR2", literal("base1")),
                                    ]),
                                },
                            ),
                            (
                                "prof2",
                                Profile {
                                    extends: set(["app1/base1"]),
                                    variables: map([
                                        ("BASE_VAR2", literal("prof2")),
                                        ("CHILD_VAR1", literal("prof2")),
                                    ]),
                                },
                            ),
                            (
                                "base2",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("BASE_VAR3", literal("base2")),
                                        ("BASE_VAR4", literal("base2")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "app2",
                    Application {
                        profiles: map([
                            (
                                "prof1",
                                Profile {
                                    extends: set(["app1/base1"]),
                                    variables: map([(
                                        "BASE_VAR2",
                                        literal("prof1"),
                                    )]),
                                },
                            ),
                            (
                                "prof3",
                                Profile {
                                    extends: set(["app2/prof1", "app1/prof2"]),
                                    variables: map([
                                        ("CHILD_VAR2", literal("prof3")),
                                        ("CHILD_VAR3", literal("prof3")),
                                    ]),
                                },
                            ),
                            (
                                "prof4",
                                Profile {
                                    extends: set(["app2/prof1"]),
                                    variables: map([
                                        ("CHILD_VAR4", literal("prof4")),
                                        ("BASE_VAR4", literal("prof4")),
                                    ]),
                                },
                            ),
                            (
                                "prof5",
                                Profile {
                                    extends: set([
                                        "app2/prof4",
                                        "app2/prof3",
                                        "app1/base2",
                                    ]),
                                    variables: map([(
                                        "CHILD_VAR5",
                                        literal("prof5"),
                                    )]),
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "app3",
                    Application {
                        profiles: map([
                            (
                                "solo",
                                Profile {
                                    extends: set([]),
                                    variables: map([(
                                        "SOLO_VAR1",
                                        literal("solo1"),
                                    )]),
                                },
                            ),
                            (
                                "striker1",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("CHILD_VAR1", literal("striker1")),
                                        ("CHILD_VAR2", literal("striker1")),
                                    ]),
                                },
                            ),
                            (
                                "striker2",
                                Profile {
                                    extends: set(["app3/striker1"]),
                                    variables: map([
                                        ("CHILD_VAR1", literal("striker2")),
                                        ("CHILD_VAR3", literal("striker2")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
            ]),
        };
        config.inherit().expect("Error resolving valid inheritance");
        assert_eq!(
            config,
            Config {
                applications: map([
                    (
                        "app1",
                        Application {
                            profiles: map([
                                (
                                    "base1",
                                    Profile {
                                        extends: set([]),
                                        variables: map([
                                            ("BASE_VAR1", literal("base1")),
                                            ("BASE_VAR2", literal("base1")),
                                        ]),
                                    },
                                ),
                                (
                                    "prof2",
                                    Profile {
                                        extends: set(["app1/base1"]),
                                        variables: map([
                                            ("BASE_VAR2", literal("prof2")),
                                            ("CHILD_VAR1", literal("prof2")),
                                            // base1
                                            ("BASE_VAR1", literal("base1")),
                                        ]),
                                    },
                                ),
                                (
                                    "base2",
                                    Profile {
                                        extends: set([]),
                                        variables: map([
                                            ("BASE_VAR3", literal("base2")),
                                            ("BASE_VAR4", literal("base2")),
                                        ]),
                                    },
                                ),
                            ]),
                        },
                    ),
                    (
                        "app2",
                        Application {
                            profiles: map([
                                (
                                    "prof1",
                                    Profile {
                                        extends: set(["app1/base1"]),
                                        variables: map([
                                            ("BASE_VAR2", literal("prof1")),
                                            ("BASE_VAR1", literal("base1")),
                                        ]),
                                    },
                                ),
                                (
                                    "prof3",
                                    Profile {
                                        extends: set([
                                            "app2/prof1",
                                            "app1/prof2"
                                        ]),
                                        variables: map([
                                            ("CHILD_VAR2", literal("prof3")),
                                            ("CHILD_VAR3", literal("prof3")),
                                            // prof1
                                            ("BASE_VAR1", literal("base1")),
                                            ("BASE_VAR2", literal("prof1")),
                                            // prof2
                                            ("CHILD_VAR1", literal("prof2")),
                                        ]),
                                    },
                                ),
                                (
                                    "prof4",
                                    Profile {
                                        extends: set(["app2/prof1"]),
                                        variables: map([
                                            ("CHILD_VAR4", literal("prof4")),
                                            ("BASE_VAR4", literal("prof4")),
                                            // prof1
                                            ("BASE_VAR2", literal("prof1")),
                                            ("BASE_VAR1", literal("base1")),
                                        ]),
                                    },
                                ),
                                (
                                    "prof5",
                                    Profile {
                                        extends: set([
                                            "app2/prof4",
                                            "app2/prof3",
                                            "app1/base2",
                                        ]),
                                        variables: map([
                                            ("CHILD_VAR5", literal("prof5"),),
                                            // prof4
                                            ("CHILD_VAR4", literal("prof4")),
                                            ("BASE_VAR4", literal("prof4")),
                                            ("BASE_VAR2", literal("prof1")),
                                            ("BASE_VAR1", literal("base1")),
                                            // prof3
                                            ("CHILD_VAR2", literal("prof3")),
                                            ("CHILD_VAR3", literal("prof3")),
                                            ("CHILD_VAR1", literal("prof2")),
                                            // base2
                                            ("BASE_VAR3", literal("base2")),
                                        ]),
                                    },
                                ),
                            ]),
                        },
                    ),
                    (
                        "app3",
                        Application {
                            profiles: map([
                                (
                                    "solo",
                                    Profile {
                                        extends: set([]),
                                        variables: map([(
                                            "SOLO_VAR1",
                                            literal("solo1"),
                                        )]),
                                    },
                                ),
                                (
                                    "striker1",
                                    Profile {
                                        extends: set([]),
                                        variables: map([
                                            ("CHILD_VAR1", literal("striker1")),
                                            ("CHILD_VAR2", literal("striker1")),
                                        ]),
                                    },
                                ),
                                (
                                    "striker2",
                                    Profile {
                                        extends: set(["app3/striker1"]),
                                        variables: map([
                                            ("CHILD_VAR1", literal("striker2")),
                                            ("CHILD_VAR3", literal("striker2")),
                                            // striker1
                                            ("CHILD_VAR2", literal("striker1")),
                                        ]),
                                    },
                                ),
                            ]),
                        },
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_inherit_cycle() {
        // One-node cycle
        let mut config = Config {
            applications: map([(
                "app1",
                Application {
                    profiles: map([(
                        "child1",
                        Profile {
                            extends: set(["app1/child1"]),
                            variables: map([]),
                        },
                    )]),
                },
            )]),
        };
        assert_eq!(
            config
                .inherit()
                .expect_err("Expected error for inheritance cycle")
                .to_string(),
            "Inheritance cycle detected: app1/child1 -> app1/child1"
        );

        // Two-node cycle
        let mut config = Config {
            applications: map([(
                "app1",
                Application {
                    profiles: map([
                        (
                            "child1",
                            Profile {
                                extends: set(["app1/child2"]),
                                variables: map([]),
                            },
                        ),
                        (
                            "child2",
                            Profile {
                                extends: set(["app1/child1"]),
                                variables: map([]),
                            },
                        ),
                    ]),
                },
            )]),
        };
        assert_eq!(
        config
            .inherit()
            .expect_err("Expected error for inheritance cycle")
            .to_string(),
        "Inheritance cycle detected: app1/child2 -> app1/child1 -> app1/child2"
    );

        // 3-node cycle
        let mut config = Config {
            applications: map([(
                "app1",
                Application {
                    profiles: map([
                        (
                            "child1",
                            Profile {
                                extends: set(["app1/child3"]),
                                variables: map([]),
                            },
                        ),
                        (
                            "child2",
                            Profile {
                                extends: set(["app1/child1"]),
                                variables: map([]),
                            },
                        ),
                        (
                            "child3",
                            Profile {
                                extends: set(["app1/child2"]),
                                variables: map([]),
                            },
                        ),
                    ]),
                },
            )]),
        };
        assert_eq!(
        config
            .inherit()
            .expect_err("Expected error for inheritance cycle")
            .to_string(),
        "Inheritance cycle detected: app1/child3 -> app1/child2 -> app1/child1 -> app1/child3"
    );
    }

    #[test]
    fn test_inherit_unknown() {
        let mut config = Config {
            applications: map([(
                "app1",
                Application {
                    profiles: map([(
                        "child1",
                        Profile {
                            extends: set(["app1/base"]),
                            variables: map([]),
                        },
                    )]),
                },
            )]),
        };

        assert_eq!(
            config
                .inherit()
                .expect_err("Expected error for unknown path")
                .to_string(),
            "Unknown profile: app1/base"
        );
    }
}
