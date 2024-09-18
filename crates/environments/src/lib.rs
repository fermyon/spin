use anyhow::{anyhow, Context};

mod environment_definition;
mod loader;

use environment_definition::{load_environment, TargetEnvironment, TriggerType};
pub use loader::ResolutionContext;
use loader::{load_and_resolve_all, ComponentToValidate};

pub async fn validate_application_against_environment_ids(
    env_ids: &[impl AsRef<str>],
    app: &spin_manifest::schema::v2::AppManifest,
    resolution_context: &ResolutionContext,
) -> anyhow::Result<Vec<anyhow::Error>> {
    if env_ids.is_empty() {
        return Ok(Default::default());
    }

    let envs = join_all_result(env_ids.iter().map(load_environment)).await?;
    validate_application_against_environments(&envs, app, resolution_context).await
}

async fn validate_application_against_environments(
    envs: &[TargetEnvironment],
    app: &spin_manifest::schema::v2::AppManifest,
    resolution_context: &ResolutionContext,
) -> anyhow::Result<Vec<anyhow::Error>> {
    use futures::FutureExt;

    for trigger_type in app.triggers.keys() {
        if let Some(env) = envs.iter().find(|e| !e.supports_trigger_type(trigger_type)) {
            anyhow::bail!(
                "Environment {} does not support trigger type {trigger_type}",
                env.name()
            );
        }
    }

    let components_by_trigger_type_futs = app.triggers.iter().map(|(ty, ts)| {
        load_and_resolve_all(app, ts, resolution_context)
            .map(|css| css.map(|css| (ty.to_owned(), css)))
    });
    let components_by_trigger_type = join_all_result(components_by_trigger_type_futs)
        .await
        .context("Failed to prepare components for target environment checking")?;

    let mut errs = vec![];

    for (trigger_type, component) in components_by_trigger_type {
        for component in &component {
            errs.extend(
                validate_component_against_environments(envs, &trigger_type, component).await?,
            );
        }
    }

    Ok(errs)
}

async fn validate_component_against_environments(
    envs: &[TargetEnvironment],
    trigger_type: &TriggerType,
    component: &ComponentToValidate<'_>,
) -> anyhow::Result<Vec<anyhow::Error>> {
    let mut errs = vec![];

    for env in envs {
        let worlds = env.worlds(trigger_type);
        if let Some(e) = validate_wasm_against_any_world(env, &worlds, component)
            .await
            .err()
        {
            errs.push(e);
        }
    }

    if errs.is_empty() {
        tracing::info!(
            "Validated component {} {} against all target worlds",
            component.id(),
            component.source_description()
        );
    }

    Ok(errs)
}

async fn validate_wasm_against_any_world(
    env: &TargetEnvironment,
    world_names: &[String],
    component: &ComponentToValidate<'_>,
) -> anyhow::Result<()> {
    let mut result = Ok(());
    for target_world in world_names {
        tracing::debug!(
            "Trying component {} {} against target world {target_world}",
            component.id(),
            component.source_description(),
        );
        match validate_wasm_against_world(env, target_world, component).await {
            Ok(()) => {
                tracing::info!(
                    "Validated component {} {} against target world {target_world}",
                    component.id(),
                    component.source_description(),
                );
                return Ok(());
            }
            Err(e) => {
                // Record the error, but continue in case a different world succeeds
                tracing::info!(
                    "Rejecting component {} {} for target world {target_world} because {e:?}",
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
    env: &TargetEnvironment,
    target_world: &str,
    component: &ComponentToValidate<'_>,
) -> anyhow::Result<()> {
    // Because we are abusing a composition tool to do validation, we have to
    // provide a name by which to refer to the component in the dummy composition.
    let component_name = "root:component";
    let component_key = wac_types::BorrowedPackageKey::from_name_and_version(component_name, None);

    // wac is going to get the world from the environment package bytes.
    // This constructs a key for that mapping.
    let env_pkg_name = env.package_namespaced_name();
    let env_pkg_key =
        wac_types::BorrowedPackageKey::from_name_and_version(&env_pkg_name, env.package_version());

    let env_name = env.name();

    let wac_text = format!(
        r#"
    package validate:component@1.0.0 targets {target_world};
    let c = new {component_name} {{ ... }};
    export c...;
    "#
    );

    let doc = wac_parser::Document::parse(&wac_text)
        .context("Internal error constructing WAC document for target checking")?;

    // TODO: if we end up needing the registry, we need to do this dance
    // for things we are providing separately, or the registry will try to
    // hoover them up and will fail.
    // let mut refpkgs = wac_resolver::packages(&doc)?;
    // refpkgs.shift_remove(&env_pkg_key);
    // refpkgs.shift_remove(&component_key);

    // TODO: determine if this is needed in circumstances other than the simple test
    // let reg_resolver = wac_resolver::RegistryPackageResolver::new(Some("wa.dev"), None).await?;
    // let mut packages = reg_resolver
    //     .resolve(&refpkgs)
    //     .await
    //     .context("reg_resolver.resolve failed")?;

    let mut packages: indexmap::IndexMap<wac_types::BorrowedPackageKey, Vec<u8>> =
        Default::default();

    packages.insert(env_pkg_key, env.package_bytes().to_vec());
    packages.insert(component_key, component.wasm_bytes().to_vec());

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
            Err(anyhow!("Component {} ({}) can't run in environment {env_name} because world {target_world} requires an export named {export}, which the component does not provide", component.id(), component.source_description()))
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
