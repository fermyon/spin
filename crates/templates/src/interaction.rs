// use console::style;
use dialoguer::Input;

use crate::template::{TemplateParameter, TemplateParameterDataType};

pub(crate) fn prompt_parameter(parameter: &TemplateParameter) -> Option<String> {
    let prompt = parameter.prompt();
    let default_value = parameter.default_value();

    loop {
        let input = match parameter.data_type() {
            TemplateParameterDataType::String(_) => ask_free_text(prompt, default_value),
        };

        match input {
            Ok(text) => match parameter.validate_value(text) {
                Ok(text) => return Some(text),
                Err(e) => {
                    println!("Invalid value: {}", e);
                }
            },
            Err(e) => {
                println!("Invalid value: {}", e);
            }
        }
    }
}

fn ask_free_text(prompt: &str, default_value: &Option<String>) -> anyhow::Result<String> {
    let mut input = Input::<String>::new();
    input.with_prompt(prompt);
    if let Some(s) = default_value {
        input.default(s.to_owned());
    }
    let result = input.interact_text()?;
    Ok(result)
}
