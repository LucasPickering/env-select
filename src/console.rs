use dialoguer::{theme::ColorfulTheme, Input, Select};

/// Show a prompt that allows the user to select a value for a variable, from
/// a given list. The user can also select a "Custom" option to enter their own
/// value.
pub fn prompt_value(
    variable: &str,
    values: &[String],
) -> anyhow::Result<String> {
    let theme = ColorfulTheme::default();
    // Show a prompt to ask the user which value to use
    let chosen_index = Select::with_theme(&theme)
        .with_prompt(format!("{}=", variable))
        .items(values)
        .item("Custom")
        .default(0)
        .interact()?;

    let value: String = if chosen_index == values.len() {
        // Let user input custom value
        Input::with_theme(&theme)
            .with_prompt(format!("{}=", variable))
            .interact_text()?
    } else {
        // This index is safe because it came from the value array above
        values[chosen_index].to_owned()
    };

    Ok(value)
}
