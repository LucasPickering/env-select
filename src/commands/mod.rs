//! All CLI subcommands are defined here. One sub-module per subcommand. Common
//! components that are specific to subcommands (and not the CLI as a whole) are
//! in this root module.

use crate::{
    commands::{
        init::InitCommand, run::RunCommand, set::SetCommand, show::ShowCommand,
    },
    config::{Config, Name, Profile},
    console::prompt_options,
    environment::Environment,
    execute::apply_side_effects,
    shell::{Shell, ShellKind},
    GlobalArgs,
};
use clap::Subcommand;
use std::path::PathBuf;

mod init;
mod run;
mod set;
mod show;

/// Subcommand to execute
#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    Init(InitCommand),
    Run(RunCommand),
    Set(SetCommand),
    Show(ShowCommand),
}

impl Commands {
    /// Execute a non-TUI command
    pub fn execute(self, global: GlobalArgs) -> anyhow::Result<()> {
        let context = CommandContext::new(global.source_file, global.shell)?;
        match self {
            Self::Init(command) => command.execute(context),
            Self::Run(command) => command.execute(context),
            Self::Set(command) => command.execute(context),
            Self::Show(command) => command.execute(context),
        }
    }
}

/// An executable subcommand. This trait isn't strictly necessary because we do
/// static dispatch via the command enum, but it's helpful to enforce a
/// consistent interface for each subcommand.
trait SubcommandTrait {
    /// Execute the subcommand
    fn execute(self, context: CommandContext) -> anyhow::Result<()>;
}

/// Arguments required for any subcommand that allows applcation/profile
/// selection.
#[derive(Clone, Debug, clap::Args)]
pub struct Selection {
    /// Application to select a profile for. If omitted, an interactive prompt
    /// will be shown to select between possible options
    pub application: Option<Name>,

    /// Profile to select. If omitted, an interactive prompt will be shown to
    /// select between possible options.
    pub profile: Option<Name>,
}

/// Data container with helper methods for all CLI subcommands
struct CommandContext {
    source_file: Option<PathBuf>,
    config: Config,
    shell: Shell,
}

impl CommandContext {
    fn new(
        source_file: Option<PathBuf>,
        shell_kind: Option<ShellKind>,
    ) -> anyhow::Result<Self> {
        // This handler will put the terminal cursor back if the user ctrl-c's
        // during the interactive dialogue
        // https://github.com/mitsuhiko/dialoguer/issues/77
        ctrlc::set_handler(move || {
            let term = dialoguer::console::Term::stdout();
            let _ = term.show_cursor();
        })?;

        let config = Config::load()?;
        let shell = match shell_kind {
            Some(kind) => Shell::from_kind(kind),
            None => Shell::detect()?,
        };

        Ok(Self {
            source_file,
            config,
            shell,
        })
    }

    /// Select an application+profile, based on user input. For both application
    /// and profile, if a default name was given, then that will be used.
    /// Otherwise, the user will be prompted to select an option via  TUI.
    fn select_profile<'a>(
        &'a self,
        selection: &'a Selection,
    ) -> anyhow::Result<&'a Profile> {
        let application = prompt_options(
            &self.config.applications,
            selection.application.as_ref(),
        )?;
        prompt_options(&application.profiles, selection.profile.as_ref())
    }

    /// Build an [Environment] from a profile. This will also run pre-setup and
    /// post-setup side effects.
    fn load_environment(
        &self,
        profile: &Profile,
    ) -> anyhow::Result<Environment> {
        // Run pre- and post-resolution side effects
        apply_side_effects(
            &profile.pre_export,
            &self.shell,
            &Environment::default(),
        )?;
        let environment = Environment::from_profile(&self.shell, profile)?;
        apply_side_effects(&profile.post_export, &self.shell, &environment)?;

        Ok(environment)
    }
}
