use crate::{get_manifest_file_name, PLUGIN_MANIFESTS_DIRECTORY_NAME};
use anyhow::Result;
use std::{fs, path::PathBuf};
pub struct PluginUninstaller {
    plugin_name: String,
    plugins_dir: PathBuf,
}

impl PluginUninstaller {
    pub fn new(plugin_name: &str, plugins_dir: PathBuf) -> Self {
        Self {
            plugin_name: plugin_name.to_owned(),
            plugins_dir,
        }
    }
    pub fn run(&self) -> Result<()> {
        // Check if plugin is installed
        let manifest_file = self
            .plugins_dir
            .join(PLUGIN_MANIFESTS_DIRECTORY_NAME)
            .join(get_manifest_file_name(&self.plugin_name));
        let plugin_exists = manifest_file.exists();
        match plugin_exists {
            // Remove the manifest and the plugin installation directory
            true => {
                fs::remove_file(manifest_file)?;
                fs::remove_dir_all(self.plugins_dir.join(&self.plugin_name))?;
                println!("Uninstalled plugin {} successfully", &self.plugin_name);
            }
            false => {
                println!(
                    "The following plugin  \"{}\" does not exist, therefore cannot be uninstalled",
                    self.plugin_name
                );
            }
        }
        Ok(())
    }
}
