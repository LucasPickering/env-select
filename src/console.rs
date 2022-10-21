use dialoguer::{theme::ColorfulTheme, Select};
use std::collections::HashMap;

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

/// Show a prompt that allows the user to select a value for a variable, from
/// a given list. The user can also select a "Custom" option to enter their own
/// value. Returns `Ok(None)` iff the user quits out of the prompt.
pub fn prompt_variable_set(
    variable_sets: &[HashMap<String, String>],
) -> anyhow::Result<&HashMap<String, String>> {
    let theme = ColorfulTheme::default();
    // Show a prompt to ask the user which value to use
    let chosen_index = Select::with_theme(&theme)
        .items(
            variable_sets
                .iter()
                .map(|variable_set| {
                    variable_set
                        .iter()
                        .map(|(variable, value)| format!("{variable}={value}"))
                        .collect::<Vec<_>>()
                        .join("\n  ")
                })
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .default(0)
        .interact()?;

    // This index is safe because it came from the value array above
    Ok(&variable_sets[chosen_index])
}
