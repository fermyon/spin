#![deny(missing_docs)]

use ::bindle::{Invoice};

use super::*;
use crate::schema::parcel::{AppManifest, ComponentManifest, RawWasmConfig};

pub(crate) fn parse(
    manifest: AppManifest,
    invoice: &Invoice,
    bindle_reader: &BindleReader,
    bindle_server_url: &str,
) -> Configuration<CoreComponent> {
    // TODO: this could do with more parsing and correctness checking
    let components = manifest
        .components
        .iter()
        .map(|c| parse_component(c, bindle_reader, invoice))
        .collect();
    let info = parse_app_info(invoice, bindle_server_url, manifest.trigger);
    Configuration {
        info,
        components,
    }
}

fn parse_component(
    source: &ComponentManifest,
    reader: &BindleReader,
    invoice: &Invoice,
) -> CoreComponent {
    CoreComponent {
        source: parse_module_source(reader, &source.source),
        id: source.id.clone(),
        wasm: parse_wasm_config(&source.wasm, reader, &invoice),
        trigger: source.trigger.clone(),
    }
}

fn parse_wasm_config(
    source: &RawWasmConfig,
    reader: &BindleReader,
    invoice: &Invoice,
) -> WasmConfig {
    let files = match &source.files {
        None => ReferencedFiles::None,
        Some(group) => {
            let parcels = bindle_utils::parcels_in_group(invoice, group);
            ReferencedFiles::BindleParcels(
                reader.clone(),
                invoice.bindle.id.clone(),
                parcels
            )
        },
    };
    WasmConfig {
        environment: source.environment.clone().unwrap_or_default(),
        files,
        allowed_http_hosts: source.allowed_http_hosts.clone().unwrap_or_default(),
    }
}

fn parse_module_source(reader: &BindleReader, parcel_id: &str) -> ModuleSource {
    ModuleSource::Bindle(BindleComponentSource {
        reader: reader.clone(),
        parcel: parcel_id.to_owned(),
    })
}

fn parse_app_info(invoice: &bindle::Invoice, bindle_server_url: &str, trigger: ApplicationTrigger) -> ApplicationInformation {
    let invoice_id = invoice.bindle.id.clone();
    let origin = ApplicationOrigin::Bindle(invoice_id, bindle_server_url.to_owned());
    ApplicationInformation {
        name: invoice.bindle.id.name().to_owned(),
        version: invoice.bindle.id.version().to_string(),  // TODO: should we enforce that a Spin version is a semver?  We can parse for the file case
        description: invoice.bindle.description.clone(),
        authors: invoice.bindle.authors.clone().unwrap_or_default(),
        trigger: trigger,
        namespace: None,
        origin,
    }
}
