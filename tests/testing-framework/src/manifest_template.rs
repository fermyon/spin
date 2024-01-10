use anyhow::Context as _;
use std::{path::Path, sync::OnceLock};

use crate::TestEnvironment;

/// A manifest template with template variables that can be substituted.
pub struct ManifestTemplate {
    manifest: String,
}

static TEMPLATE: OnceLock<regex::Regex> = OnceLock::new();
impl ManifestTemplate {
    /// Read a manifest template from a file.
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let manifest = std::fs::read_to_string(path)
            .with_context(|| format!("could not read manifest template at '{}'", path.display()))?;
        Ok(Self { manifest })
    }

    /// Substitute template variables in the manifest template.
    pub fn substitute<R>(&mut self, env: &mut TestEnvironment<R>) -> Result<(), anyhow::Error> {
        let regex = TEMPLATE.get_or_init(|| regex::Regex::new(r"%\{(.*?)\}").unwrap());
        while let Some(captures) = regex.captures(&self.manifest) {
            let (Some(full), Some(capture)) = (captures.get(0), captures.get(1)) else {
                continue;
            };
            let template = capture.as_str();
            let (template_key, template_value) = template.split_once('=').with_context(|| {
                format!("invalid template '{template}'(template should be in the form $KEY=$VALUE)")
            })?;
            let replacement = match template_key.trim() {
                "source" => {
                    let component_binary = std::path::PathBuf::from(
                        test_components::path(template_value)
                            .with_context(|| format!("no such component '{template_value}'"))?,
                    );
                    let wasm_name = component_binary.file_name().unwrap().to_str().unwrap();
                    env.copy_into(&component_binary, wasm_name)?;
                    wasm_name.to_owned()
                }
                "port" => {
                    let guest_port = template_value
                        .parse()
                        .with_context(|| format!("failed to parse '{template_value}' as port"))?;
                    let port = env
                        .get_port(guest_port)?
                        .with_context(|| format!("no port {guest_port} exposed by any service"))?;
                    port.to_string()
                }
                _ => {
                    anyhow::bail!("unknown template key: {template_key}");
                }
            };
            self.manifest.replace_range(full.range(), &replacement);
        }
        Ok(())
    }

    pub fn contents(&self) -> &str {
        &self.manifest
    }
}
