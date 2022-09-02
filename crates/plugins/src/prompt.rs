use anyhow::Result;
use std::io;

/// Prompts user as to whether they trust the source of the plugin and
/// want to proceed with installation.
pub(crate) struct Prompter {
    plugin_name: String,
    plugin_license: String,
    source_url: String,
}

impl Prompter {
    /// Creates a new prompter
    pub fn new(plugin_name: &str, plugin_license: &str, source_url: &str) -> Result<Self> {
        Ok(Self {
            plugin_name: plugin_name.to_string(),
            plugin_license: plugin_license.to_string(),
            source_url: source_url.to_string(),
        })
    }

    fn print_prompt(&self) {
        println!(
            "Installing plugin {} with license {} from {}\n",
            self.plugin_name, self.plugin_license, self.source_url
        );
        println!("Are you sure you want to proceed? (y/N)");
    }

    fn are_you_sure(&self) -> Result<bool> {
        let mut resp = String::new();
        io::stdin().read_line(&mut resp)?;
        Ok(self.parse_response(&mut resp))
    }

    fn parse_response(&self, resp: &mut str) -> bool {
        let resp = resp.trim().to_lowercase();
        resp.eq("yes") || resp.eq("y")
    }

    /// Returns whether or not the user would like to proceed with the installation of a plugin.
    pub(crate) fn run(&self) -> Result<bool> {
        self.print_prompt();
        self.are_you_sure()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_response() {
        let p = Prompter::new(
            "best-plugin",
            "MIT",
            "www.example.com/releases/example-1.0.tar.gz",
        )
        .unwrap();
        let mut resp = String::from("\n\t  yes   ");
        assert!(p.parse_response(&mut resp));
    }
}
