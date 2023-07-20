use crate::config::{Application, MapExt, Name, Profile};
use anyhow::bail;
use atty::Stream;
use dialoguer::{theme::ColorfulTheme, Select};
use indexmap::IndexMap;
use std::fmt::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prompt the user to select one option from a list.
pub fn prompt_options<'a, T: Prompt>(
    options: &'a IndexMap<Name, T>,
    default_name: Option<&'a Name>,
) -> anyhow::Result<&'a T> {
    match default_name {
        Some(default_name) => options.try_get(default_name),

        // Show a prompt to ask the user which profile to use
        None => {
            let theme = ColorfulTheme::default();
            let options_vec = options.iter().collect::<Vec<_>>();

            if options_vec.is_empty() {
                bail!("No {}s to choose from", T::SELF_NAME);
            }

            // Show a prompt to ask the user which value to use
            let chosen_index = Select::with_theme(&theme)
                .with_prompt(format!("Select {}", T::SELF_NAME))
                .items(
                    options_vec
                        .iter()
                        .map(|(name, option)| option.format_option(name))
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
                .default(0)
                .interact()?;

            // This index is safe because it came from the value array above
            Ok(options_vec[chosen_index].1)
        }
    }
}

/// Print the given message, but only if we're connected to a TTY. Normally we
/// avoid printing anything to stdout to avoid conflict with shell commands, but
/// if we're on a TTY, we know the output isn't being piped so it's safe to
/// print here.
pub fn print_hint(message: &str) -> anyhow::Result<()> {
    if atty::is(Stream::Stdout) {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(
            ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true),
        )?;
        println!("{message}");
        stdout.reset()?;
    }
    Ok(())
}

/// Print a friendly hint reminding the user to configure their shell
pub fn print_installation_hint() -> anyhow::Result<()> {
    print_hint(&format!(
        "Initialize env-select automatically on shell startup: \
            {REPOSITORY}/tree/v{VERSION}#configure-your-shell",
    ))
}

pub trait Prompt: Sized {
    const SELF_NAME: &'static str;

    fn format_option(&self, name: &Name) -> String;
}

impl Prompt for Application {
    const SELF_NAME: &'static str = "application";

    fn format_option(&self, name: &Name) -> String {
        // First line of the output will be the profile name, then
        // we'll show all the variable mappings
        let mut buffer = String::new();
        writeln!(buffer, "=== {name} ===").unwrap();
        for profile_name in self.profiles.keys() {
            writeln!(buffer, "{profile_name}").unwrap();
        }
        buffer
    }
}

impl Prompt for Profile {
    const SELF_NAME: &'static str = "profile";

    fn format_option(&self, name: &Name) -> String {
        // First line of the output will be the profile name, then
        // we'll show all the variable mappings
        let mut buffer = String::new();
        writeln!(buffer, "=== {name} ===").unwrap();
        for (variable, value) in &self.variables {
            writeln!(buffer, "{variable} = {value}").unwrap();
        }
        buffer
    }
}
