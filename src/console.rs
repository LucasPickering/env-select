use crate::config::{Application, Profile};
use anyhow::bail;
use dialoguer::{theme::ColorfulTheme, Select};

/// Show a prompt that allows the user to select a value for a variable, from
/// a given list. The user can also select a "Custom" option to enter their own
/// value. Returns `Ok(None)` iff the user quits out of the prompt.
pub fn prompt_variable<'a>(
    variable: &str,
    options: &'a [String],
) -> anyhow::Result<&'a str> {
    let theme = ColorfulTheme::default();
    // Show a prompt to ask the user which value to use
    let chosen_index = Select::with_theme(&theme)
        .items(
            options
                .iter()
                .map(|value| format!("{variable}={value}"))
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .default(0)
        .interact()?;

    // This index is safe because it came from the value array above
    Ok(&options[chosen_index])
}

/// Show a prompt that allows the user to select a profile for an application,
/// from a given list.
pub fn prompt_application(
    application: &Application,
) -> anyhow::Result<&Profile> {
    let theme = ColorfulTheme::default();
    let profiles: Vec<(&String, &Profile)> =
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
