//! Utilitied related to profile inheritance resolution

use crate::config::{Config, MapExt, Profile, ProfileReference};
use anyhow::{anyhow, bail};
use indexmap::{IndexMap, IndexSet};
use log::trace;
use std::{collections::HashMap, fmt::Display, hash::Hash};

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

        // The "intuitive" way to resolve inheritance is to start with the
        // first parent, and merge in values bottom-up. It turns out the easiest
        // way is really to do it in reverse: start with the child, and merge
        // parents right-to-left, giving precedence to what's *already in the
        // profile*. For vecs, precedence means lower down the list.
        for parent in parents.iter().rev() {
            trace!("Resolving parent {reference} -> {parent}");

            // Check for cycles
            if visited.contains(parent) {
                bail!("Inheritance cycle detected: {}", display_cycle(visited));
            }

            // Check if parent needs to be resolved. If parent is an unknown
            // path, we'll skip over here and fail down below
            if let Some(grandparents) = self.unresolved.swap_remove(parent) {
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
            trace!("Merging values from {parent} into {reference}");
            let parent = (*self
                .profiles
                .get(parent)
                .ok_or_else(|| anyhow!("Unknown profile: {}", parent))?)
            .clone();
            let child = self
                .profiles
                .get_mut(&reference)
                .ok_or_else(|| anyhow!("Unknown profile: {}", reference))?;
            child.inherit_from(parent);
        }
        Ok(())
    }
}

trait Inherit {
    /// Merge a parent into this child. For map-like fields, the child's entries
    /// will take precedence. For list-like fields, the parent will be appended
    /// to the *beginning* of the child.
    fn inherit_from(&mut self, parent: Self);
}

impl Inherit for Profile {
    fn inherit_from(&mut self, parent: Self) {
        self.variables.inherit_from(parent.variables);
        self.pre_export.inherit_from(parent.pre_export);
        self.post_export.inherit_from(parent.post_export);
    }
}

impl<K: Hash + Eq + PartialEq, V> Inherit for IndexMap<K, V> {
    fn inherit_from(&mut self, mut parent: Self) {
        // Start with the parent and add children in, to get parent->child
        // ordering
        parent.extend(self.drain(..));
        *self = parent;
    }
}

impl<T> Inherit for Vec<T> {
    fn inherit_from(&mut self, mut parent: Self) {
        // Effectively merge the parent at the beginning of the vec
        parent.append(self);
        *self = parent;
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
    use crate::{
        config::Profile,
        test_util::{config, literal, map, set, side_effect},
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_inherit_single() {
        let mut cfg = config(vec![(
            "app",
            vec![
                (
                    "base",
                    Profile {
                        extends: set([]),
                        pre_export: vec![side_effect("base pre", "base pre")],
                        post_export: vec![side_effect(
                            "base post",
                            "base post",
                        )],
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

                        pre_export: vec![side_effect("child pre", "child pre")],
                        post_export: vec![side_effect(
                            "child post",
                            "child post",
                        )],
                        variables: map([
                            ("VAR1", literal("child")),
                            // VAR2 comes from base
                            ("VAR3", literal("child")),
                        ]),
                    },
                ),
            ],
        )]);
        cfg.inherit().expect("Error resolving valid inheritance");
        assert_eq!(
            cfg,
            config(vec![(
                "app",
                vec![
                    (
                        "base",
                        Profile {
                            extends: set([]),
                            pre_export: vec![side_effect(
                                "base pre", "base pre",
                            )],
                            post_export: vec![side_effect(
                                "base post",
                                "base post",
                            )],
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
                            pre_export: vec![
                                side_effect("base pre", "base pre"),
                                side_effect("child pre", "child pre"),
                            ],
                            post_export: vec![
                                side_effect("base post", "base post"),
                                side_effect("child post", "child post"),
                            ],
                            variables: map([
                                ("VAR1", literal("child")),
                                ("VAR3", literal("child")),
                                ("VAR2", literal("base")),
                            ]),
                        },
                    ),
                ]
            )])
        );
    }

    #[test]
    fn test_inherit_linear() {
        let mut cfg = config(vec![
            (
                "app1",
                vec![
                    (
                        "base",
                        Profile {
                            extends: set([]),
                            pre_export: vec![side_effect(
                                "base pre", "base pre",
                            )],
                            post_export: vec![side_effect(
                                "base post",
                                "base post",
                            )],
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
                            pre_export: vec![side_effect(
                                "child1 pre",
                                "child1 pre",
                            )],
                            post_export: vec![side_effect(
                                "child1 post",
                                "child1 post",
                            )],
                            variables: map([
                                ("VAR1", literal("child1")),
                                // VAR2 comes from base
                                ("VAR3", literal("child1")),
                            ]),
                        },
                    ),
                ],
            ),
            (
                "app2",
                vec![(
                    "child2",
                    Profile {
                        extends: set(["app1/child1"]),
                        pre_export: vec![side_effect(
                            "child2 pre",
                            "child2 pre",
                        )],
                        post_export: vec![side_effect(
                            "child2 post",
                            "child2 post",
                        )],
                        variables: map([
                            ("VAR1", literal("child2")),
                            // VAR2 comes from base
                            // VAR3 comes from child1
                            ("VAR4", literal("child2")),
                        ]),
                    },
                )],
            ),
        ]);
        cfg.inherit().expect("Error resolving valid inheritance");
        assert_eq!(
            cfg,
            config(vec![
                (
                    "app1",
                    vec![
                        (
                            "base",
                            Profile {
                                extends: set([]),
                                pre_export: vec![side_effect(
                                    "base pre", "base pre"
                                )],
                                post_export: vec![side_effect(
                                    "base post",
                                    "base post",
                                )],
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
                                pre_export: vec![
                                    side_effect("base pre", "base pre"),
                                    side_effect("child1 pre", "child1 pre"),
                                ],
                                post_export: vec![
                                    side_effect("base post", "base post"),
                                    side_effect("child1 post", "child1 post"),
                                ],
                                variables: map([
                                    ("VAR2", literal("base")),
                                    ("VAR1", literal("child1")),
                                    ("VAR3", literal("child1")),
                                ]),
                            },
                        ),
                    ],
                ),
                (
                    "app2",
                    vec![(
                        "child2",
                        Profile {
                            extends: set(["app1/child1"]),
                            pre_export: vec![
                                side_effect("base pre", "base pre"),
                                side_effect("child1 pre", "child1 pre"),
                                side_effect("child2 pre", "child2 pre"),
                            ],
                            post_export: vec![
                                side_effect("base post", "base post"),
                                side_effect("child1 post", "child1 post"),
                                side_effect("child2 post", "child2 post"),
                            ],
                            variables: map([
                                ("VAR2", literal("base")),
                                ("VAR1", literal("child2")),
                                ("VAR4", literal("child2")),
                                ("VAR3", literal("child1")),
                            ]),
                        },
                    )],
                ),
            ]),
        );
    }

    /// This test cases uses a complex inheritance graph to catch bugs around
    /// multiple inheritance, multi-layer inheritance, multiple independent
    /// graphs, etc. In reality this kind of inheritance would be useless
    /// because side effects get duplicated so much, but it's good to have it be
    /// well-defined.
    #[test]
    fn test_inherit_nonlinear() {
        let mut cfg = config(vec![
            (
                "app1",
                vec![
                    (
                        "base1",
                        Profile {
                            extends: set([]),
                            pre_export: vec![side_effect(
                                "base1 pre",
                                "base1 pre",
                            )],
                            post_export: vec![side_effect(
                                "base1 post",
                                "base1 post",
                            )],
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
                            pre_export: vec![side_effect(
                                "prof2 pre",
                                "prof2 pre",
                            )],
                            post_export: vec![side_effect(
                                "prof2 post",
                                "prof2 post",
                            )],
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
                            pre_export: vec![side_effect(
                                "base2 pre",
                                "base2 pre",
                            )],
                            post_export: vec![side_effect(
                                "base2 post",
                                "base2 post",
                            )],
                            variables: map([
                                ("BASE_VAR3", literal("base2")),
                                ("BASE_VAR4", literal("base2")),
                            ]),
                        },
                    ),
                ],
            ),
            (
                "app2",
                vec![
                    (
                        "prof1",
                        Profile {
                            extends: set(["app1/base1"]),
                            pre_export: vec![side_effect(
                                "prof1 pre",
                                "prof1 pre",
                            )],
                            post_export: vec![side_effect(
                                "prof1 post",
                                "prof1 post",
                            )],
                            variables: map([("BASE_VAR2", literal("prof1"))]),
                        },
                    ),
                    (
                        "prof3",
                        Profile {
                            extends: set(["app2/prof1", "app1/prof2"]),
                            pre_export: vec![side_effect(
                                "prof3 pre",
                                "prof3 pre",
                            )],
                            post_export: vec![side_effect(
                                "prof3 post",
                                "prof3 post",
                            )],
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
                            pre_export: vec![side_effect(
                                "prof4 pre",
                                "prof4 pre",
                            )],
                            post_export: vec![side_effect(
                                "prof4 post",
                                "prof4 post",
                            )],
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
                            pre_export: vec![side_effect(
                                "prof5 pre",
                                "prof5 pre",
                            )],
                            post_export: vec![side_effect(
                                "prof5 post",
                                "prof5 post",
                            )],
                            variables: map([("CHILD_VAR5", literal("prof5"))]),
                        },
                    ),
                ],
            ),
            (
                "app3",
                vec![
                    (
                        "solo",
                        Profile {
                            extends: set([]),
                            pre_export: vec![side_effect(
                                "solo pre", "solo pre",
                            )],
                            post_export: vec![side_effect(
                                "solo post",
                                "solo post",
                            )],
                            variables: map([("SOLO_VAR1", literal("solo1"))]),
                        },
                    ),
                    (
                        "striker1",
                        Profile {
                            extends: set([]),
                            pre_export: vec![side_effect(
                                "striker1 pre",
                                "striker1 pre",
                            )],
                            post_export: vec![side_effect(
                                "striker1 post",
                                "striker1 post",
                            )],
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
                            pre_export: vec![side_effect(
                                "striker2 pre",
                                "striker2 pre",
                            )],
                            post_export: vec![side_effect(
                                "striker2 post",
                                "striker2 post",
                            )],
                            variables: map([
                                ("CHILD_VAR1", literal("striker2")),
                                ("CHILD_VAR3", literal("striker2")),
                            ]),
                        },
                    ),
                ],
            ),
        ]);
        cfg.inherit().expect("Error resolving valid inheritance");
        assert_eq!(
            cfg,
            config(vec![
                (
                    "app1",
                    vec![
                        (
                            "base1",
                            Profile {
                                extends: set([]),
                                pre_export: vec![side_effect(
                                    "base1 pre",
                                    "base1 pre"
                                )],
                                post_export: vec![side_effect(
                                    "base1 post",
                                    "base1 post"
                                )],
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
                                pre_export: vec![
                                    side_effect("base1 pre", "base1 pre"),
                                    side_effect("prof2 pre", "prof2 pre"),
                                ],
                                post_export: vec![
                                    side_effect("base1 post", "base1 post"),
                                    side_effect("prof2 post", "prof2 post"),
                                ],
                                variables: map([
                                    // base1
                                    ("BASE_VAR1", literal("base1")),
                                    // me
                                    ("BASE_VAR2", literal("prof2")),
                                    ("CHILD_VAR1", literal("prof2")),
                                ]),
                            },
                        ),
                        (
                            "base2",
                            Profile {
                                extends: set([]),
                                pre_export: vec![side_effect(
                                    "base2 pre",
                                    "base2 pre"
                                )],
                                post_export: vec![side_effect(
                                    "base2 post",
                                    "base2 post"
                                )],
                                variables: map([
                                    ("BASE_VAR3", literal("base2")),
                                    ("BASE_VAR4", literal("base2")),
                                ]),
                            },
                        ),
                    ],
                ),
                (
                    "app2",
                    vec![
                        (
                            "prof1",
                            Profile {
                                extends: set(["app1/base1"]),
                                pre_export: vec![
                                    side_effect("base1 pre", "base1 pre"),
                                    side_effect("prof1 pre", "prof1 pre"),
                                ],
                                post_export: vec![
                                    side_effect("base1 post", "base1 post"),
                                    side_effect("prof1 post", "prof1 post"),
                                ],
                                variables: map([
                                    ("BASE_VAR1", literal("base1")),
                                    ("BASE_VAR2", literal("prof1")),
                                ]),
                            },
                        ),
                        (
                            "prof3",
                            Profile {
                                extends: set(["app2/prof1", "app1/prof2"]),
                                pre_export: vec![
                                    // prof1
                                    side_effect("base1 pre", "base1 pre"),
                                    side_effect("prof1 pre", "prof1 pre"),
                                    // prof2
                                    side_effect("base1 pre", "base1 pre"),
                                    side_effect("prof2 pre", "prof2 pre"),
                                    // me
                                    side_effect("prof3 pre", "prof3 pre"),
                                ],
                                post_export: vec![
                                    // prof1
                                    side_effect("base1 post", "base1 post"),
                                    side_effect("prof1 post", "prof1 post"),
                                    // prof2
                                    side_effect("base1 post", "base1 post"),
                                    side_effect("prof2 post", "prof2 post"),
                                    // me
                                    side_effect("prof3 post", "prof3 post"),
                                ],
                                variables: map([
                                    // prof1
                                    ("BASE_VAR1", literal("base1")),
                                    // prof2
                                    ("BASE_VAR2", literal("prof2")),
                                    ("CHILD_VAR1", literal("prof2")),
                                    // me
                                    ("CHILD_VAR2", literal("prof3")),
                                    ("CHILD_VAR3", literal("prof3")),
                                ]),
                            },
                        ),
                        (
                            "prof4",
                            Profile {
                                extends: set(["app2/prof1"]),
                                pre_export: vec![
                                    // prof1
                                    side_effect("base1 pre", "base1 pre"),
                                    side_effect("prof1 pre", "prof1 pre"),
                                    // me
                                    side_effect("prof4 pre", "prof4 pre"),
                                ],
                                post_export: vec![
                                    // prof1
                                    side_effect("base1 post", "base1 post"),
                                    side_effect("prof1 post", "prof1 post"),
                                    // me
                                    side_effect("prof4 post", "prof4 post"),
                                ],
                                variables: map([
                                    // prof1
                                    ("BASE_VAR1", literal("base1")),
                                    ("BASE_VAR2", literal("prof1")),
                                    // me
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
                                pre_export: vec![
                                    // prof4
                                    side_effect("base1 pre", "base1 pre"),
                                    side_effect("prof1 pre", "prof1 pre"),
                                    side_effect("prof4 pre", "prof4 pre"),
                                    // prof3
                                    side_effect("base1 pre", "base1 pre"),
                                    side_effect("prof1 pre", "prof1 pre"),
                                    side_effect("base1 pre", "base1 pre"),
                                    side_effect("prof2 pre", "prof2 pre"),
                                    side_effect("prof3 pre", "prof3 pre"),
                                    // base2
                                    side_effect("base2 pre", "base2 pre"),
                                    // me
                                    side_effect("prof5 pre", "prof5 pre"),
                                ],
                                post_export: vec![
                                    // prof4
                                    side_effect("base1 post", "base1 post"),
                                    side_effect("prof1 post", "prof1 post"),
                                    side_effect("prof4 post", "prof4 post"),
                                    // prof3
                                    side_effect("base1 post", "base1 post"),
                                    side_effect("prof1 post", "prof1 post"),
                                    side_effect("base1 post", "base1 post"),
                                    side_effect("prof2 post", "prof2 post"),
                                    side_effect("prof3 post", "prof3 post"),
                                    // base2
                                    side_effect("base2 post", "base2 post"),
                                    // me
                                    side_effect("prof5 post", "prof5 post"),
                                ],
                                variables: map([
                                    // prof4
                                    ("BASE_VAR1", literal("base1")),
                                    ("BASE_VAR2", literal("prof2")),
                                    ("CHILD_VAR4", literal("prof4")),
                                    // prof3
                                    ("CHILD_VAR1", literal("prof2")),
                                    ("CHILD_VAR2", literal("prof3")),
                                    ("CHILD_VAR3", literal("prof3")),
                                    // base2
                                    ("BASE_VAR3", literal("base2")),
                                    ("BASE_VAR4", literal("base2")),
                                    // me
                                    ("CHILD_VAR5", literal("prof5")),
                                ]),
                            },
                        ),
                    ],
                ),
                (
                    "app3",
                    vec![
                        (
                            "solo",
                            Profile {
                                extends: set([]),
                                pre_export: vec![side_effect(
                                    "solo pre", "solo pre"
                                )],
                                post_export: vec![side_effect(
                                    "solo post",
                                    "solo post"
                                )],
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
                                pre_export: vec![side_effect(
                                    "striker1 pre",
                                    "striker1 pre"
                                ),],
                                post_export: vec![side_effect(
                                    "striker1 post",
                                    "striker1 post"
                                ),],
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
                                pre_export: vec![
                                    side_effect("striker1 pre", "striker1 pre"),
                                    side_effect("striker2 pre", "striker2 pre"),
                                ],
                                post_export: vec![
                                    side_effect(
                                        "striker1 post",
                                        "striker1 post"
                                    ),
                                    side_effect(
                                        "striker2 post",
                                        "striker2 post"
                                    ),
                                ],
                                variables: map([
                                    // striker1
                                    ("CHILD_VAR2", literal("striker1")),
                                    // me
                                    ("CHILD_VAR1", literal("striker2")),
                                    ("CHILD_VAR3", literal("striker2")),
                                ]),
                            },
                        ),
                    ],
                ),
            ]),
        );
    }

    #[test]
    fn test_inherit_cycle() {
        // One-node cycle
        let mut cfg = config(vec![(
            "app1",
            vec![(
                "child1",
                Profile {
                    extends: set(["app1/child1"]),
                    pre_export: vec![],
                    post_export: vec![],
                    variables: map([]),
                },
            )],
        )]);
        assert_eq!(
            cfg.inherit()
                .expect_err("Expected error for inheritance cycle")
                .to_string(),
            "Inheritance cycle detected: app1/child1 -> app1/child1"
        );

        // Two-node cycle
        let mut cfg = config(vec![(
            "app1",
            vec![
                (
                    "child1",
                    Profile {
                        extends: set(["app1/child2"]),
                        pre_export: vec![],
                        post_export: vec![],
                        variables: map([]),
                    },
                ),
                (
                    "child2",
                    Profile {
                        extends: set(["app1/child1"]),
                        pre_export: vec![],
                        post_export: vec![],
                        variables: map([]),
                    },
                ),
            ],
        )]);
        assert_eq!(
        cfg
            .inherit()
            .expect_err("Expected error for inheritance cycle")
            .to_string(),
        "Inheritance cycle detected: app1/child2 -> app1/child1 -> app1/child2"
    );

        // 3-node cycle
        let mut cfg = config(vec![(
            "app1",
            vec![
                (
                    "child1",
                    Profile {
                        extends: set(["app1/child3"]),
                        pre_export: vec![],
                        post_export: vec![],
                        variables: map([]),
                    },
                ),
                (
                    "child2",
                    Profile {
                        extends: set(["app1/child1"]),
                        pre_export: vec![],
                        post_export: vec![],
                        variables: map([]),
                    },
                ),
                (
                    "child3",
                    Profile {
                        extends: set(["app1/child2"]),
                        pre_export: vec![],
                        post_export: vec![],
                        variables: map([]),
                    },
                ),
            ],
        )]);
        assert_eq!(
        cfg
            .inherit()
            .expect_err("Expected error for inheritance cycle")
            .to_string(),
        "Inheritance cycle detected: app1/child3 -> app1/child2 -> app1/child1 -> app1/child3"
    );
    }

    #[test]
    fn test_inherit_unknown() {
        let mut cfg = config(vec![(
            "app1",
            vec![(
                "child1",
                Profile {
                    extends: set(["app1/base"]),
                    pre_export: vec![],
                    post_export: vec![],
                    variables: map([]),
                },
            )],
        )]);

        assert_eq!(
            cfg.inherit()
                .expect_err("Expected error for unknown path")
                .to_string(),
            "Unknown profile: app1/base"
        );
    }
}
