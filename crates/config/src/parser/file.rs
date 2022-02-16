#![deny(missing_docs)]

use super::*;
use crate::schema::file::{AppManifest, ComponentManifest, RawWasmConfig, RawModuleSource, AppInformation};

pub(crate) fn parse(manifest: AppManifest, source_path: impl AsRef<Path>) -> Configuration<CoreComponent> {
    // TODO: this could do with more parsing and correctness checking
    let app_directory = source_path
        .as_ref()
        .parent()
        .expect("The application file did not have a parent directory");
    let components = manifest
        .components
        .iter()
        .map(|c| parse_component(c, &app_directory))
        .collect();
    let info = parse_app_info(manifest.info, &source_path);
    Configuration {
        info,
        components,
    }
}

fn parse_component(source: &ComponentManifest, app_directory: &Path) -> CoreComponent {
    CoreComponent {
        source: parse_module_source(&source.source),
        id: source.id.clone(),
        wasm: parse_wasm_config(&source.wasm, app_directory),
        trigger: source.trigger.clone(),
    }
}

fn parse_wasm_config(source: &RawWasmConfig, app_directory: &Path) -> WasmConfig {
    let files = match &source.files {
        None => ReferencedFiles::None,
        Some(patterns) => ReferencedFiles::FilePatterns(
            app_directory.to_owned(),
            patterns.clone(),
        ),
    };
    WasmConfig {
        environment: source.environment.clone().unwrap_or_default(),
        files,
        allowed_http_hosts: source.allowed_http_hosts.clone().unwrap_or_default(),
    }
}

fn parse_module_source(source: &RawModuleSource) -> ModuleSource {
    match source {
        RawModuleSource::Bindle(_) =>
            panic!("Bindle module sources are not yet supported in file-based app config"),
        RawModuleSource::FileReference(path) =>
            ModuleSource::FileReference(path.clone())
    }
}

fn parse_app_info(raw: AppInformation, source_path: impl AsRef<Path>) -> ApplicationInformation {
    let origin = ApplicationOrigin::File(source_path.as_ref().to_owned());
    ApplicationInformation {
        name: raw.name,
        version: raw.version,
        description: raw.description,
        authors: raw.authors.unwrap_or_default(),
        trigger: raw.trigger,
        namespace: raw.namespace,
        origin,
    }
}
