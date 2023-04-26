// TODO: subset of spin_templates::interaction

use dialoguer::Confirm;

pub(crate) fn confirm(text: &str) -> std::io::Result<bool> {
    Confirm::new().with_prompt(text).interact()
}
