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
        match &mut self.0.kind {
            ValueSourceKind::File { path } => {
                path.qualify(context.config_path);
            }
            ValueSourceKind::Command { cwd: Some(cwd), .. } => {
                cwd.qualify(context.config_path);
            }
            _ => {}
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
            Some(directory) => directory.join(self.as_path()),
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
    use super::*;
    use crate::{
        config::Profile,
        test_util::{command, config, file, map, set},
    };
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    const CONFIG_PATH: &str = "/root/.env-select.toml";

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
        cfg.qualify(CONFIG_PATH);
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
    fn test_qualify_command_cwd_path() {
        let mut cfg = config(vec![(
            "app",
            vec![(
                "prof",
                Profile {
                    variables: map([
                        ("VAR1", command("echo")),
                        ("VAR2", command("echo").cwd(".venv/bin")),
                    ]),
                    ..Default::default()
                },
            )],
        )]);
        cfg.qualify(CONFIG_PATH);
        assert_eq!(
            cfg,
            config(vec![(
                "app",
                vec![(
                    "prof",
                    Profile {
                        variables: map([
                            ("VAR1", command("echo")),
                            ("VAR2", command("echo").cwd("/root/.venv/bin")),
                        ]),
                        ..Default::default()
                    },
                )],
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
                    variables: map([("VAR1", file("var.txt"))]),
                    ..Default::default()
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
                        variables: map([("VAR1", file("/root/var.txt"))]),
                        ..Default::default()
                    },
                )],
            )])
        );
    }

    /// Detailed test cases for qualifying file paths
    #[rstest]
    // Directories
    #[case(".", "/root")]
    #[case("..", "/root/..")]
    #[case(".venv", "/root/.venv")]
    // Files
    #[case("var.txt", "/root/var.txt")]
    #[case("../var.txt", "/root/../var.txt")]
    #[case("data/var.txt", "/root/data/var.txt")]
    #[case("/usr/var.txt", "/usr/var.txt")]
    fn test_qualify_path(#[case] path: &str, #[case] expected: &str) {
        let mut path = PathBuf::from(path);
        let expected = PathBuf::from(expected);
        path.qualify(&PathBuf::from(CONFIG_PATH));
        assert_eq!(path, expected);
    }
}
