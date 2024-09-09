use anyhow::{anyhow, Context};

mod environment_definition;
mod loader;

use environment_definition::{TargetEnvironment, TargetWorld, TriggerType};
pub use loader::ResolutionContext;
use loader::{load_and_resolve_all, ComponentToValidate};

pub async fn validate_application_against_environment_ids(
    env_ids: impl Iterator<Item = &str>,
    app: &spin_manifest::schema::v2::AppManifest,
    resolution_context: &ResolutionContext,
) -> anyhow::Result<()> {
    let envs = join_all_result(env_ids.map(resolve_environment_id)).await?;
    validate_application_against_environments(&envs, app, resolution_context).await
}

async fn resolve_environment_id(id: &str) -> anyhow::Result<TargetEnvironment> {
    let (name, ver) = id.split_once('@').ok_or(anyhow!(
        "Target environment '{id}' does not specify a version"
    ))?;
    let client = oci_distribution::Client::default();
    let auth = oci_distribution::secrets::RegistryAuth::Anonymous;
    let env_def_ref =
        oci_distribution::Reference::try_from(format!("ghcr.io/itowlson/spinenvs/{name}:{ver}"))?;
    let (man, _digest) = client
        .pull_manifest(&env_def_ref, &auth)
        .await
        .with_context(|| format!("Failed to find environment '{id}' in registry"))?;
    let im = match man {
        oci_distribution::manifest::OciManifest::Image(im) => im,
        oci_distribution::manifest::OciManifest::ImageIndex(_ind) => {
            anyhow::bail!("Environment '{id}' definition is unusable - stored in registry in incorrect format")
        }
    };
    let the_layer = &im.layers[0];
    let mut out = Vec::with_capacity(the_layer.size.try_into().unwrap_or_default());
    client
        .pull_blob(&env_def_ref, the_layer, &mut out)
        .await
        .with_context(|| {
            format!("Failed to download environment '{id}' definition from registry")
        })?;
    let te = serde_json::from_slice(&out).with_context(|| {
        format!("Failed to load environment '{id}' definition - invalid JSON schema")
    })?;
    Ok(te)
}

pub async fn validate_application_against_environments(
    envs: &[TargetEnvironment],
    app: &spin_manifest::schema::v2::AppManifest,
    resolution_context: &ResolutionContext,
) -> anyhow::Result<()> {
    use futures::FutureExt;

    for trigger_type in app.triggers.keys() {
        if let Some(env) = envs
            .iter()
            .find(|e| !e.environments.contains_key(trigger_type))
        {
            anyhow::bail!(
                "Environment {} does not support trigger type {trigger_type}",
                env.name
            );
        }
    }

    let components_by_trigger_type_futs = app.triggers.iter().map(|(ty, ts)| {
        load_and_resolve_all(app, ts, resolution_context)
            .map(|css| css.map(|css| (ty.to_owned(), css)))
    });
    let components_by_trigger_type = join_all_result(components_by_trigger_type_futs).await?;

    for (trigger_type, component) in components_by_trigger_type {
        for component in &component {
            validate_component_against_environments(envs, &trigger_type, component).await?;
        }
    }

    Ok(())
}

async fn validate_component_against_environments(
    envs: &[TargetEnvironment],
    trigger_type: &TriggerType,
    component: &ComponentToValidate<'_>,
) -> anyhow::Result<()> {
    let worlds = envs
        .iter()
        .map(|e| {
            e.environments
                .get(trigger_type)
                .ok_or(anyhow!(
                    "Environment '{}' doesn't support trigger type {trigger_type}",
                    e.name
                ))
                .map(|w| (e.name.as_str(), w))
        })
        .collect::<Result<std::collections::HashSet<_>, _>>()?;
    validate_component_against_worlds(worlds.into_iter(), component).await?;
    Ok(())
}

async fn validate_component_against_worlds(
    target_worlds: impl Iterator<Item = (&str, &TargetWorld)>,
    component: &ComponentToValidate<'_>,
) -> anyhow::Result<()> {
    for (env_name, target_world) in target_worlds {
        validate_wasm_against_any_world(env_name, target_world, component).await?;
    }

    tracing::info!(
        "Validated component {} {} against all target worlds",
        component.id(),
        component.source_description()
    );
    Ok(())
}

async fn validate_wasm_against_any_world(
    env_name: &str,
    target_world: &TargetWorld,
    component: &ComponentToValidate<'_>,
) -> anyhow::Result<()> {
    let mut result = Ok(());
    for target_str in target_world.versioned_names() {
        tracing::info!(
            "Trying component {} {} against target world {target_str}",
            component.id(),
            component.source_description(),
        );
        match validate_wasm_against_world(env_name, &target_str, component).await {
            Ok(()) => {
                tracing::info!(
                    "Validated component {} {} against target world {target_str}",
                    component.id(),
                    component.source_description(),
                );
                return Ok(());
            }
            Err(e) => {
                // Record the error, but continue in case a different world succeeds
                tracing::info!(
                    "Rejecting component {} {} for target world {target_str} because {e:?}",
                    component.id(),
                    component.source_description(),
                );
                result = Err(e);
            }
        }
    }
    result
}

async fn validate_wasm_against_world(
    env_name: &str,
    target_str: &str,
    component: &ComponentToValidate<'_>,
) -> anyhow::Result<()> {
    let comp_name = "root:component";

    let wac_text = format!(
        r#"
    package validate:component@1.0.0 targets {target_str};
    let c = new {comp_name} {{ ... }};
    export c...;
    "#
    );

    let doc = wac_parser::Document::parse(&wac_text)?;

    let compkey = wac_types::BorrowedPackageKey::from_name_and_version(comp_name, None);

    let mut refpkgs = wac_resolver::packages(&doc)?;
    refpkgs.retain(|k, _| k != &compkey);

    let reg_resolver = wac_resolver::RegistryPackageResolver::new(Some("wa.dev"), None).await?;
    let mut packages = reg_resolver
        .resolve(&refpkgs)
        .await
        .context("reg_resolver.resolve failed")?;

    packages.insert(compkey, component.wasm_bytes().to_vec());

    match doc.resolve(packages) {
        Ok(_) => Ok(()),
        Err(wac_parser::resolution::Error::TargetMismatch { kind, name, world, .. }) => {
            // This one doesn't seem to get hit at the moment - we get MissingTargetExport or ImportNotInTarget instead
            Err(anyhow!("Component {} ({}) can't run in environment {env_name} because world {world} expects an {} named {name}", component.id(), component.source_description(), kind.to_string().to_lowercase()))
        }
        Err(wac_parser::resolution::Error::MissingTargetExport { name, world, .. }) => {
            Err(anyhow!("Component {} ({}) can't run in environment {env_name} because world {world} requires an export named {name}, which the component does not provide", component.id(), component.source_description()))
        }
        Err(wac_parser::resolution::Error::PackageMissingExport { export, .. }) => {
            // TODO: The export here seems wrong - it seems to contain the world name rather than the interface name
            Err(anyhow!("Component {} ({}) can't run in environment {env_name} because world {target_str} requires an export named {export}, which the component does not provide", component.id(), component.source_description()))
        }
        Err(wac_parser::resolution::Error::ImportNotInTarget { name, world, .. }) => {
            Err(anyhow!("Component {} ({}) can't run in environment {env_name} because world {world} does not provide an import named {name}, which the component requires", component.id(), component.source_description()))
        }
        Err(e) => {
            Err(anyhow!(e))
        },
    }
}

/// Equivalent to futures::future::join_all, but specialised for iterators of
/// fallible futures. It returns a Result<Vec<...>> instead of a Vec<Result<...>> -
/// this just moves the transposition boilerplate out of the main flow.
async fn join_all_result<T, I>(iter: I) -> anyhow::Result<Vec<T>>
where
    I: IntoIterator,
    I::Item: std::future::Future<Output = anyhow::Result<T>>,
{
    let vec_result = futures::future::join_all(iter).await;
    vec_result.into_iter().collect()
}
