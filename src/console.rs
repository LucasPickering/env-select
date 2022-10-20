use dialoguer::{theme::ColorfulTheme, Input, Select};

/// Show a prompt that allows the user to select a value for a variable, from
/// a given list. The user can also select a "Custom" option to enter their own
/// value. Returns `Ok(None)` iff the user quits out of the prompt.
pub fn prompt_options(
    variable: &str,
    options: &[String],
) -> anyhow::Result<Option<String>> {
    let theme = ColorfulTheme::default();
    // Show a prompt to ask the user which value to use
    let chosen_index_opt = Select::with_theme(&theme)
        .with_prompt(format!("{}=", variable))
        .items(options)
        .item("Custom")
        .default(0)
        .interact_opt()?;
    let chosen_index = match chosen_index_opt {
        Some(value) => value,
        None => return Ok(None),
    };

    let value: String = if chosen_index == options.len() {
        // Let user input custom value
        Input::with_theme(&theme)
            .with_prompt(format!("{}=", variable))
            .interact_text()?
    } else {
        // This index is safe because it came from the value array above
        options[chosen_index].to_owned()
    };

    Ok(Some(value))
}
