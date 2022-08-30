mod git;
// TODO: just export PluginInstaller
pub mod install;
mod plugin_manifest;
mod prompt;
pub mod uninstall;
pub mod version_check;

/// Directory where the manifests of installed plugins are stored.
pub const PLUGIN_MANIFESTS_DIRECTORY_NAME: &str = "manifests";

fn get_manifest_file_name(plugin_name: &str) -> String {
    format!("{}.json", plugin_name)
}

// Given a name and option version, outputs expected file name for the plugin.
fn get_manifest_file_name_version(plugin_name: &str, version: &Option<semver::Version>) -> String {
    match version {
        Some(v) => format!("{}@{}.json", plugin_name, v),
        None => get_manifest_file_name(plugin_name),
    }
}
