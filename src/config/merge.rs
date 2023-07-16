use super::{Application, Config, Profile};
use indexmap::{map::Entry, IndexMap, IndexSet};
use std::hash::Hash;

/// Indicates that two values of this type can be merged together.
pub trait Merge {
    /// Merge another value into this one. The "other" value **will take
    /// precedence** over this one, meaning conflicting values from the incoming
    /// will overwrite.
    fn merge(&mut self, other: Self);
}

impl Merge for Config {
    fn merge(&mut self, other: Self) {
        self.applications.merge(other.applications);
    }
}

impl Merge for Application {
    fn merge(&mut self, other: Self) {
        self.profiles.merge(other.profiles)
    }
}

impl Merge for Profile {
    fn merge(&mut self, other: Self) {
        // Incoming entries take priority over ours
        // TODO - should we really be merging profiles?
        self.extends.merge(other.extends);
        self.variables.extend(other.variables.into_iter());
    }
}

impl<T: Eq + Hash> Merge for IndexSet<T> {
    fn merge(&mut self, other: Self) {
        self.extend(other)
    }
}

impl<K: Eq + Hash, V: Merge> Merge for IndexMap<K, V> {
    fn merge(&mut self, other: Self) {
        for (k, other_v) in other {
            match self.entry(k) {
                Entry::Occupied(mut entry) => entry.get_mut().merge(other_v),
                Entry::Vacant(entry) => {
                    entry.insert(other_v);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        tests::{literal, map, set},
        Application, Config, Profile,
    };
    use indexmap::{IndexMap, IndexSet};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_merge_map() {
        let mut map1: IndexMap<String, IndexSet<&str>> =
            map([("a", set(["1"])), ("b", set(["2"]))]);
        let map2 = map([("a", set(["3"])), ("c", set(["4"]))]);
        map1.merge(map2);
        assert_eq!(
            map1,
            map(
                [("a", set(["1", "3"])), ("b", set(["2"])), ("c", set(["4"])),]
            )
        );
    }

    #[test]
    fn test_merge_config() {
        let mut config1 = Config {
            applications: map([
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "prof1",
                                Profile {
                                    extends: set(["prof2"]),
                                    variables: map([
                                        // Gets overwritten
                                        ("VAR1", literal("val1")),
                                        ("VAR2", literal("val2")),
                                    ]),
                                },
                            ),
                            // No conflict
                            (
                                "prof2",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("VAR1", literal("val11")),
                                        ("VAR2", literal("val22")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                // No conflict
                (
                    "app2",
                    Application {
                        profiles: map([(
                            "prof1",
                            Profile {
                                extends: set([]),
                                variables: map([("VAR1", literal("val1"))]),
                            },
                        )]),
                    },
                ),
            ]),
        };
        let config2 = Config {
            applications: map([
                // Merged into existing
                (
                    "app1",
                    Application {
                        profiles: map([
                            (
                                "prof1",
                                Profile {
                                    extends: set(["prof3"]),
                                    variables: map([
                                        // Overwrites
                                        ("VAR1", literal("val7")),
                                    ]),
                                },
                            ),
                            // No conflict
                            (
                                "prof3",
                                Profile {
                                    extends: set([]),
                                    variables: map([
                                        ("VAR1", literal("val111")),
                                        ("VAR2", literal("val222")),
                                    ]),
                                },
                            ),
                        ]),
                    },
                ),
                // No conflict
                (
                    "app3",
                    Application {
                        profiles: map([(
                            "prof1",
                            Profile {
                                extends: set([]),
                                variables: map([("VAR1", literal("val11"))]),
                            },
                        )]),
                    },
                ),
            ]),
        };
        config1.merge(config2);
        assert_eq!(
            config1,
            Config {
                applications: map([
                    (
                        "app1",
                        Application {
                            profiles: map([
                                (
                                    "prof1",
                                    Profile {
                                        extends: set(["prof2", "prof3"]),
                                        variables: map([
                                            ("VAR1", literal("val7")),
                                            ("VAR2", literal("val2")),
                                        ])
                                    }
                                ),
                                (
                                    "prof2",
                                    Profile {
                                        extends: set([]),
                                        variables: map([
                                            ("VAR1", literal("val11")),
                                            ("VAR2", literal("val22"))
                                        ])
                                    }
                                ),
                                (
                                    "prof3",
                                    Profile {
                                        extends: set([]),
                                        variables: map([
                                            ("VAR1", literal("val111")),
                                            ("VAR2", literal("val222")),
                                        ])
                                    }
                                ),
                            ]),
                        }
                    ),
                    (
                        "app2",
                        Application {
                            profiles: map([(
                                "prof1",
                                Profile {
                                    extends: set([]),
                                    variables: map([("VAR1", literal("val1"))])
                                },
                            )]),
                        }
                    ),
                    (
                        "app3",
                        Application {
                            profiles: map([(
                                "prof1",
                                Profile {
                                    extends: set([]),
                                    variables: map([(
                                        "VAR1",
                                        literal("val11")
                                    )])
                                },
                            )]),
                        }
                    ),
                ]),
            }
        );
    }
}
