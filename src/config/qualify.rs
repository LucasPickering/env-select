use crate::config::{
    Application, Config, Name, Profile, ProfileReference, ValueSource,
    ValueSourceKind,
};
use log::trace;
use std::path::{Path, PathBuf};

impl Config {
    /// Convert all paths in the config to be absolute. File paths will be
    /// resolved relative to the parent of the given path, which should be the
    /// the file from which this config was loaded. Profile references will be
    /// relative to the parent application.
    pub(super) fn qualify<P: AsRef<Path>>(&mut self, config_path: P) {
        Qualify::qualify(self, config_path.as_ref())
    }
}

/// Augment extra data onto an object, given some extra context. E.g. this is
/// used to qualify relative file paths as absolute paths.
trait Qualify<'a> {
    type Context: ?Sized;

    fn qualify(&'a mut self, context: &'a Self::Context);
}

struct ApplicationContext<'a> {
    config_path: &'a Path,
    application_name: &'a Name,
}

impl<'a> Qualify<'a> for Config {
    type Context = Path;

    fn qualify(&mut self, config_path: &Self::Context) {
        trace!("Qualifying config `{config_path:?}`");
        for (name, application) in &mut self.applications {
            trace!("Qualifying application `{name}`");
            application.qualify(&ApplicationContext {
                config_path,
                application_name: name,
            });
        }
    }
}

impl<'a> Qualify<'a> for Application {
    type Context = ApplicationContext<'a>;

    fn qualify(&mut self, context: &Self::Context) {
        for (name, profile) in &mut self.profiles {
            trace!(
                "Qualifying profile `{}/{}`",
                context.application_name,
                name
            );
            profile.qualify(context);
        }
    }
}

impl<'a> Qualify<'a> for Profile {
    type Context = ApplicationContext<'a>;

    fn qualify(&mut self, context: &Self::Context) {
        self.extends = self
            .extends
            .drain(..)
            .map(|mut parent| {
                parent.qualify(context);
                parent
            })
            .collect();

        for value_source in self.variables.values_mut() {
            value_source.qualify(context);
        }
    }
}

impl<'a> Qualify<'a> for ProfileReference {
    type Context = ApplicationContext<'a>;

    /// Qualify profile reference by ensuring application is included
    fn qualify(&mut self, context: &Self::Context) {
        if !self.is_qualified() {
            let previous_string = self.to_string();
            self.application = Some(context.application_name.clone());
            trace!(
                "Qualifyied profile reference `{previous_string}` to `{self}`"
            );
        }
    }
}

impl<'a> Qualify<'a> for ValueSource {
    type Context = ApplicationContext<'a>;

    fn qualify(&mut self, context: &Self::Context) {
        if let ValueSourceKind::File { path } = &mut self.0.kind {
            path.qualify(context.config_path);
        }
    }
}

impl<'a> Qualify<'a> for PathBuf {
    type Context = Path;

    /// Qualify file references based on the given config *file*. The
    /// parent directory of the context path will be the root of the absolute
    /// path (if this path is relative).
    fn qualify(&mut self, config_path: &Self::Context) {
        let new_path = match config_path.parent() {
            Some(directory) => directory.join(&self),
            None => panic!(
                "Qualification context should be a path to a \
                config file, but got {config_path:?}"
            ),
        };
        trace!("Qualified path {self:?} to {new_path:?}");
        *self = new_path;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::Profile,
        test_util::{config, file, map, set},
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_qualify_profile_reference() {
        let mut cfg = config(vec![(
            "app",
            vec![
                (
                    "base",
                    Profile {
                        extends: set([]),
                        ..Default::default()
                    },
                ),
                (
                    "child1",
                    Profile {
                        extends: set(["base"]),
                        ..Default::default()
                    },
                ),
                (
                    "child2",
                    Profile {
                        extends: set(["app2/base"]),
                        ..Default::default()
                    },
                ),
            ],
        )]);
        cfg.qualify("");
        assert_eq!(
            cfg,
            config(vec![(
                "app",
                vec![
                    (
                        "base",
                        Profile {
                            extends: set([]),
                            ..Default::default()
                        },
                    ),
                    (
                        "child1",
                        Profile {
                            extends: set(["app/base"]),
                            ..Default::default()
                        },
                    ),
                    (
                        "child2",
                        Profile {
                            extends: set(["app2/base"]),
                            ..Default::default()
                        },
                    ),
                ],
            )])
        );
    }

    #[test]
    fn test_qualify_file_value_path() {
        let mut cfg = config(vec![(
            "app",
            vec![(
                "prof",
                Profile {
                    extends: set([]),
                    pre_export: vec![],
                    post_export: vec![],
                    variables: map([
                        ("VAR1", file("var.txt")),
                        ("VAR2", file("../var.txt")),
                        ("VAR3", file("data/var.txt")),
                        ("VAR4", file("/usr/var.txt")),
                    ]),
                },
            )],
        )]);
        cfg.qualify("/root/.env-select.toml");
        assert_eq!(
            cfg,
            config(vec![(
                "app",
                vec![(
                    "prof",
                    Profile {
                        extends: set([]),
                        pre_export: vec![],
                        post_export: vec![],
                        variables: map([
                            ("VAR1", file("/root/var.txt")),
                            ("VAR2", file("/root/../var.txt")),
                            ("VAR3", file("/root/data/var.txt")),
                            ("VAR4", file("/usr/var.txt")),
                        ]),
                    },
                )],
            )])
        );
    }
}
