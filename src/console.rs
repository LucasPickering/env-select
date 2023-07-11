use crate::config::{Application, Name, Profile};
use anyhow::bail;
use atty::Stream;
use dialoguer::{theme::ColorfulTheme, Select};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Show a prompt that allows the user to select a profile for an application,
/// from a given list.
pub fn prompt_application(
    application: &Application,
) -> anyhow::Result<&Profile> {
    let theme = ColorfulTheme::default();
    let profiles: Vec<(&Name, &Profile)> =
        application.profiles.iter().collect();

    if profiles.is_empty() {
        bail!("No profiles for this application");
    }

    // Show a prompt to ask the user which value to use
    let chosen_index = Select::with_theme(&theme)
        .items(
            profiles
                .iter()
                .map(|(name, profile)| {
                    // First line of the output will be the profile name, then
                    // we'll show all the variable mappings
                    let mut buffer = format!("=== {name} ===\n");
                    for (variable, value) in &profile.variables {
                        buffer += &format!("{variable}={value}\n");
                    }
                    buffer
                })
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .default(0)
        .interact()?;

    // This index is safe because it came from the value array above
    Ok(profiles[chosen_index].1)
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
