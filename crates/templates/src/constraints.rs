use regex::Regex;

#[derive(Clone, Debug)]
pub(crate) struct StringConstraints {
    pub regex: Option<Regex>,
}

impl StringConstraints {
    pub fn validate(&self, text: String) -> anyhow::Result<String> {
        if let Some(regex) = self.regex.as_ref() {
            if !regex.is_match(&text) {
                anyhow::bail!("Input '{}' does not match pattern '{}'", text, regex);
            }
        }
        Ok(text)
    }
}
