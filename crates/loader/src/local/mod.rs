//! Functionality to get a prepared Spin application configuration from spin.toml.

#![deny(missing_docs)]

/// Module to prepare the assets for the components of an application.
mod assets;
/// Configuration representation for a Spin apoplication as a local spin.toml file.
mod config;

use anyhow::{anyhow, Context, Result};
use config::{RawAppInformation, RawAppManifest, RawComponentManifest};
use futures::future;
use spin_config::{
    ApplicationInformation, ApplicationOrigin, Configuration, CoreComponent, ModuleSource,
    WasmConfig,
};
use std::path::{Path, PathBuf};
use tokio::{fs::File, io::AsyncReadExt};

/// Given the path to a spin.toml manifest file, prepare its assets locally and
/// get a prepared application configuration consumable by a Spin execution context.
/// If a directory is provided, use it as the base directory to expand the assets,
/// otherwise create a new temporary directory.
pub async fn from_file(
    app: impl AsRef<Path>,
    base_dst: Option<PathBuf>,
) -> Result<Configuration<CoreComponent>> {
    let mut buf = vec![];
    File::open(app.as_ref())
        .await?
        .read_to_end(&mut buf)
        .await
        .with_context(|| anyhow!("Cannot read manifest file from {:?}", app.as_ref()))?;

    let manifest: RawAppManifest = toml::from_slice(&buf)?;

    prepare(manifest, app, base_dst).await
}

async fn prepare(
    raw: RawAppManifest,
    src: impl AsRef<Path>,
    base_dst: Option<PathBuf>,
) -> Result<Configuration<CoreComponent>> {
    let dir = match base_dst {
        Some(d) => d,
        None => tempfile::tempdir()?.into_path(),
    };
    let info = info(raw.info, &src);

    let components = future::join_all(
        raw.components
            .into_iter()
            .map(|c| async { core(c, &src, &dir).await })
            .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .map(|x| x.expect("Cannot prepare component."))
    .collect::<Vec<_>>();

    Ok(Configuration { info, components })
}

/// Given a component manifest, prepare its assets and return a fully formed core component.
async fn core(
    raw: RawComponentManifest,
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
) -> Result<CoreComponent> {
    let src = src
        .as_ref()
        .parent()
        .expect("The application file did not have a parent directory.");
    let source = match raw.source {
        config::RawModuleSource::FileReference(p) => {
            let p = match p.is_absolute() {
                true => p,
                false => src.join(p),
            };

            ModuleSource::FileReference(p)
        }
        config::RawModuleSource::Bindle(_) => {
            todo!("Bindle module sources are not yet supported in file-based app config")
        }
    };

    let id = raw.id;
    let mounts = match raw.wasm.files {
        Some(f) => vec![assets::prepare_component(&f, src, &base_dst, &id).await?],
        None => vec![],
    };
    let environment = raw.wasm.environment.unwrap_or_default();
    let allowed_http_hosts = raw.wasm.allowed_http_hosts.unwrap_or_default();
    let wasm = WasmConfig {
        environment,
        mounts,
        allowed_http_hosts,
    };
    let trigger = raw.trigger;

    Ok(CoreComponent {
        source,
        id,
        wasm,
        trigger,
    })
}

/// Convert the raw application information from the spin.toml manifest to the standard configuration.
fn info(raw: RawAppInformation, src: impl AsRef<Path>) -> ApplicationInformation {
    ApplicationInformation {
        api_version: raw.api_version,
        name: raw.name,
        version: raw.version,
        description: raw.description,
        authors: raw.authors.unwrap_or_default(),
        trigger: raw.trigger,
        namespace: raw.namespace,
        origin: ApplicationOrigin::File(src.as_ref().to_path_buf()),
    }
}

#[cfg(test)]
mod tests {
    use crate::local::config::RawModuleSource;

    use super::*;
    use anyhow::Result;
    use spin_config::{ApplicationTrigger, HttpExecutor, TriggerConfig};
    use std::path::PathBuf;

    const MANIFEST: &str = "tests/valid-with-files/spin.toml";
    #[tokio::test]
    async fn test_from_local_source() -> Result<()> {
        let dir: Option<PathBuf> = None;
        let cfg = from_file(MANIFEST, dir).await?;

        assert_eq!(cfg.info.name, "spin-local-source-test");
        assert_eq!(cfg.info.version, "1.0.0");
        assert_eq!(cfg.info.api_version, "0.1.0");
        assert_eq!(
            cfg.info.authors[0],
            "Fermyon Engineering <engineering@fermyon.com>"
        );

        let ApplicationTrigger::Http(http) = cfg.info.trigger;
        assert_eq!(http.base, "/".to_string());

        let TriggerConfig::Http(http) = cfg.components[0].trigger.clone();
        assert_eq!(http.executor.unwrap(), HttpExecutor::Spin);
        assert_eq!(http.route, "/...".to_string());

        assert_eq!(cfg.components[0].wasm.mounts.len(), 1);

        assert_eq!(
            cfg.info.origin,
            ApplicationOrigin::File("tests/valid-with-files/spin.toml".into())
        );

        Ok(())
    }

    const CFG_TEST: &str = r#"
    apiVersion  = "0.1.0"
    name        = "chain-of-command"
    version     = "6.11.2"
    description = "A simple application that returns the number of lights"
    authors     = [ "Gul Madred", "Edward Jellico", "JL" ]
    trigger     = { type = "http", base   = "/" }

    [[component]]
        source = "path/to/wasm/file.wasm"
        id     = "four-lights"
        files  = ["file.txt", "subdir/another.txt"]
    [component.trigger]
        route          = "/lights"
        executor       = "spin"
    [component.environment]
        env1 = "first"
        env2 = "second"
    
    [[component]]
        id = "abc"
    [component.source]
        reference = "bindle reference"
        parcel    = "parcel"
    [component.trigger]
        route = "/test"
    "#;

    #[test]
    fn test_manifest() -> Result<()> {
        let cfg: RawAppManifest = toml::from_str(CFG_TEST)?;

        assert_eq!(cfg.info.name, "chain-of-command");
        assert_eq!(cfg.info.version, "6.11.2");
        assert_eq!(
            cfg.info.description,
            Some("A simple application that returns the number of lights".to_string())
        );

        let ApplicationTrigger::Http(http) = cfg.info.trigger;
        assert_eq!(http.base, "/".to_string());

        assert_eq!(cfg.info.authors.unwrap().len(), 3);
        assert_eq!(cfg.components[0].id, "four-lights".to_string());

        let TriggerConfig::Http(http) = cfg.components[0].trigger.clone();
        assert_eq!(http.executor.unwrap(), HttpExecutor::Spin);
        assert_eq!(http.route, "/lights".to_string());

        let test_component = &cfg.components[0];
        let test_env = &test_component.wasm.environment.as_ref().unwrap();
        assert_eq!(test_env.len(), 2);
        assert_eq!(test_env.get("env1").unwrap(), "first");
        assert_eq!(test_env.get("env2").unwrap(), "second");

        let test_files = &test_component.wasm.files.as_ref().unwrap();
        assert_eq!(test_files.len(), 2);
        assert_eq!(test_files[0], "file.txt");
        assert_eq!(test_files[1], "subdir/another.txt");

        let b = match cfg.components[1].source.clone() {
            RawModuleSource::Bindle(b) => b,
            RawModuleSource::FileReference(_) => panic!("expected bindle source"),
        };

        assert_eq!(b.reference, "bindle reference".to_string());
        assert_eq!(b.parcel, "parcel".to_string());

        Ok(())
    }
}
