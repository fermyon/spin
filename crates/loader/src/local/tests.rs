use crate::local::config::{RawDirectoryPlacement, RawFileMount, RawModuleSource};

use super::*;
use anyhow::Result;
use spin_manifest::{HttpConfig, HttpExecutor, HttpTriggerConfiguration};
use std::path::PathBuf;

#[tokio::test]
async fn test_from_local_source() -> Result<()> {
    const MANIFEST: &str = "tests/valid-with-files/spin.toml";

    let temp_dir = tempfile::tempdir()?;
    let dir = temp_dir.path();
    let app = from_file(MANIFEST, dir, &None, false).await?;

    assert_eq!(app.info.name, "spin-local-source-test");
    assert_eq!(app.info.version, "1.0.0");
    assert_eq!(app.info.spin_version, SpinVersion::V1);
    assert_eq!(
        app.info.authors[0],
        "Fermyon Engineering <engineering@fermyon.com>"
    );

    let http: HttpTriggerConfiguration = app.info.trigger.try_into()?;
    assert_eq!(http.base, "/".to_string());

    let component = &app.components[0];
    assert_eq!(component.wasm.mounts.len(), 1);

    let http: HttpConfig = app
        .component_triggers
        .get(&component.id)
        .cloned()
        .unwrap()
        .try_into()?;
    assert_eq!(http.executor.unwrap(), HttpExecutor::Spin);
    assert_eq!(http.route, "/...".to_string());

    let expected_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/valid-with-files/spin.toml");
    assert_eq!(app.info.origin, ApplicationOrigin::File(expected_path));

    Ok(())
}

#[test]
fn test_manifest() -> Result<()> {
    const MANIFEST: &str = include_str!("../../tests/valid-manifest.toml");

    let cfg_any: RawAppManifestAnyVersion = toml::from_str(MANIFEST)?;
    let RawAppManifestAnyVersion::V1(cfg) = cfg_any;

    assert_eq!(cfg.info.name, "chain-of-command");
    assert_eq!(cfg.info.version, "6.11.2");
    assert_eq!(
        cfg.info.description,
        Some("A simple application that returns the number of lights".to_string())
    );

    let http: HttpTriggerConfiguration = cfg.info.trigger.try_into()?;
    assert_eq!(http.base, "/".to_string());

    assert_eq!(cfg.info.authors.unwrap().len(), 3);
    assert_eq!(cfg.components[0].id, "four-lights".to_string());

    let http: HttpConfig = cfg.components[0].trigger.clone().try_into()?;
    assert_eq!(http.executor.unwrap(), HttpExecutor::Spin);
    assert_eq!(http.route, "/lights".to_string());

    let test_component = &cfg.components[0];
    let test_env = &test_component.wasm.environment.as_ref().unwrap();
    assert_eq!(test_env.len(), 2);
    assert_eq!(test_env.get("env1").unwrap(), "first");
    assert_eq!(test_env.get("env2").unwrap(), "second");

    let test_files = &test_component.wasm.files.as_ref().unwrap();
    assert_eq!(test_files.len(), 3);
    assert_eq!(test_files[0], RawFileMount::Pattern("file.txt".to_owned()));
    assert_eq!(
        test_files[1],
        RawFileMount::Placement(RawDirectoryPlacement {
            source: PathBuf::from("valid-with-files"),
            destination: PathBuf::from("/vwf"),
        })
    );
    assert_eq!(
        test_files[2],
        RawFileMount::Pattern("subdir/another.txt".to_owned())
    );

    let b = match cfg.components[1].source.clone() {
        RawModuleSource::Bindle(b) => b,
        RawModuleSource::FileReference(_) => panic!("expected bindle source"),
    };

    assert_eq!(b.reference, "bindle reference".to_string());
    assert_eq!(b.parcel, "parcel".to_string());

    Ok(())
}

#[test]
fn test_unknown_version_is_rejected() {
    const MANIFEST: &str = include_str!("../../tests/invalid-version.toml");

    let cfg = toml::from_str::<RawAppManifestAnyVersion>(MANIFEST);
    assert!(
        cfg.is_err(),
        "Expected version to be validated but it wasn't"
    );

    let e = cfg.unwrap_err().to_string();
    assert!(
        e.contains("spin_version"),
        "Expected error to mention `spin_version`"
    );
}

#[test]
fn test_wagi_executor_with_custom_entrypoint() -> Result<()> {
    const MANIFEST: &str = include_str!("../../tests/wagi-custom-entrypoint.toml");

    const EXPECTED_CUSTOM_ENTRYPOINT: &str = "custom-entrypoint";
    const EXPECTED_DEFAULT_ARGV: &str = "${SCRIPT_NAME} ${ARGS}";

    let cfg_any: RawAppManifestAnyVersion = toml::from_str(MANIFEST)?;
    let RawAppManifestAnyVersion::V1(cfg) = cfg_any;

    let http_config: HttpConfig = cfg.components[0].trigger.clone().try_into()?;

    match http_config.executor.as_ref().unwrap() {
        HttpExecutor::Spin => panic!("expected wagi http executor"),
        HttpExecutor::Wagi(spin_manifest::WagiConfig { entrypoint, argv }) => {
            assert_eq!(entrypoint, EXPECTED_CUSTOM_ENTRYPOINT);
            assert_eq!(argv, EXPECTED_DEFAULT_ARGV);
        }
    };

    Ok(())
}

#[tokio::test]
async fn test_duplicate_component_id_is_rejected() -> Result<()> {
    const MANIFEST: &str = "tests/invalid-manifest-duplicate-id.toml";

    let temp_dir = tempfile::tempdir()?;
    let dir = temp_dir.path();
    let app = from_file(MANIFEST, dir, &None, false).await;

    assert!(
        app.is_err(),
        "Expected component IDs to be unique, but there were duplicates"
    );

    let e = app.unwrap_err().to_string();
    assert!(
        e.contains("hello"),
        "Expected error to contain duplicate component ID `hello`"
    );

    Ok(())
}

#[tokio::test]
async fn test_insecure_allow_all_with_invalid_url() -> Result<()> {
    const MANIFEST: &str = "tests/insecure-allow-all-with-invalid-url.toml";

    let temp_dir = tempfile::tempdir()?;
    let dir = temp_dir.path();
    let app = from_file(MANIFEST, dir, &None, false).await;

    assert!(
        app.is_ok(),
        "Expected insecure:allow-all can skip url validation"
    );

    Ok(())
}

#[tokio::test]
async fn test_invalid_url_in_allowed_http_hosts_is_rejected() -> Result<()> {
    const MANIFEST: &str = "tests/invalid-url-in-allowed-http-hosts.toml";

    let temp_dir = tempfile::tempdir()?;
    let dir = temp_dir.path();
    let app = from_file(MANIFEST, dir, &None, false).await;

    assert!(
        app.is_err(),
        "Expected url can be parsed by reqwest Url, but there was invalid url"
    );

    let e = app.unwrap_err().to_string();
    assert!(
        e.contains("some-random-api.ml"),
        "Expected error to contain invalid url `some-random-api.ml`"
    );

    Ok(())
}
