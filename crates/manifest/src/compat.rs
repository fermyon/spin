//! Compatibility for old manifest versions.

use crate::{
    error::Error,
    schema::{v1, v2},
};

/// Converts a V1 app manifest to V2.
pub fn v1_to_v2_app(manifest: v1::AppManifestV1) -> Result<v2::AppManifest, Error> {
    let trigger_type = manifest.trigger.trigger_type.clone();
    let trigger_global_configs = [(trigger_type.clone(), manifest.trigger.config)]
        .into_iter()
        .collect();

    let application = v2::AppDetails {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        authors: manifest.authors,
        trigger_global_configs,
    };

    let app_variables = manifest
        .variables
        .into_iter()
        .map(|(key, var)| Ok((id_from_string(key)?, var)))
        .collect::<Result<_, Error>>()?;

    let mut triggers = v2::Map::<String, Vec<v2::Trigger>>::default();
    let mut components = v2::Map::default();
    for component in manifest.components {
        let component_id = component_id_from_string(component.id)?;

        let variables = component
            .config
            .into_iter()
            .map(|(key, var)| Ok((id_from_string(key)?, var)))
            .collect::<Result<_, Error>>()?;

        let key_value_stores = component
            .key_value_stores
            .into_iter()
            .map(id_from_string)
            .collect::<Result<_, Error>>()?;

        let sqlite_databases = component
            .sqlite_databases
            .into_iter()
            .map(id_from_string)
            .collect::<Result<_, Error>>()?;

        let ai_models = component
            .ai_models
            .into_iter()
            .map(id_from_string)
            .collect::<Result<_, Error>>()?;

        components.insert(
            component_id.clone(),
            v2::Component {
                source: component.source,
                description: component.description,
                variables,
                environment: component.environment,
                files: component.files,
                exclude_files: component.exclude_files,
                allowed_http_hosts: component.allowed_http_hosts,
                key_value_stores,
                sqlite_databases,
                ai_models,
                build: component.build,
            },
        );
        triggers
            .entry(trigger_type.clone())
            .or_default()
            .push(v2::Trigger {
                id: format!("trigger-{component_id}"),
                component: Some(v2::ComponentSpec::Reference(component_id)),
                components: Default::default(),
                config: component.trigger,
            });
    }
    Ok(v2::AppManifest {
        spin_manifest_version: Default::default(),
        application,
        variables: app_variables,
        triggers,
        components,
    })
}

fn component_id_from_string(id: String) -> Result<v2::KebabId, Error> {
    // If it's already valid, do nothing
    if let Ok(id) = id.clone().try_into() {
        return Ok(id);
    }
    // Fix two likely problems; under_scores and mixedCase
    let id = id.replace('_', "-").to_lowercase();
    id.clone()
        .try_into()
        .map_err(|err: String| Error::InvalidID { id, reason: err })
}

fn id_from_string<const DELIM: char>(id: String) -> Result<v2::Id<DELIM>, Error> {
    id.clone()
        .try_into()
        .map_err(|err: String| Error::InvalidID { id, reason: err })
}
