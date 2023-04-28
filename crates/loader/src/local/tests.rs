use crate::local::config::{RawDirectoryPlacement, RawFileMount, RawModuleSource};

use super::*;
use anyhow::Result;
use spin_manifest::{HttpConfig, HttpExecutor, HttpTriggerConfiguration};
use std::path::PathBuf;

fn raw_manifest_from_str(toml: &str) -> Result<RawAppManifestAnyVersion> {
    raw_manifest_from_slice(toml.as_bytes())
}

#[tokio::test]
async fn test_from_local_source() -> Result<()> {
    const MANIFEST: &str = "tests/valid-with-files/spin.toml";

    let temp_dir = tempfile::tempdir()?;
    let dir = temp_dir.path();
    let app = from_file(MANIFEST, Some(dir)).await?;

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
fn test_manifest_v1_signatures() -> Result<()> {
    const MANIFEST: &str = include_str!("../../tests/valid-manifest.toml");
    let ageless = MANIFEST
        .replace("spin_version", "temp")
        .replace("spin_manifest_version", "temp");
    let v1_old = ageless.replace("temp", "spin_version");
    let v1_new = ageless.replace("temp", "spin_manifest_version");
    into_v1_manifest(&v1_old, "chain-of-command")?;
    into_v1_manifest(&v1_new, "chain-of-command")?;
    Ok(())
}

fn into_v1_manifest(spin_versioned_manifest: &str, app_name: &str) -> Result<()> {
    use config::RawAppManifestAnyVersionImpl;
    let raw: RawAppManifestAnyVersionImpl<toml::Value> =
        toml::from_slice(spin_versioned_manifest.as_bytes())?;
    let manifest = raw.into_v1();
    assert_eq!(manifest.info.name, app_name);
    Ok(())
}

#[test]
fn test_manifest() -> Result<()> {
    const MANIFEST: &str = include_str!("../../tests/valid-manifest.toml");

    let cfg_any: RawAppManifestAnyVersion = raw_manifest_from_str(MANIFEST)?;
    let cfg = cfg_any.into_v1();

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

    let u = match cfg.components[2].source.clone() {
        RawModuleSource::Url(u) => u,
        RawModuleSource::FileReference(_) => panic!("expected URL source"),
    };

    assert_eq!(u.url, "https://example.com/wasm.wasm.wasm".to_string());
    assert_eq!(u.digest, "sha256:12345".to_string());

    Ok(())
}

#[tokio::test]
async fn can_parse_url_sources() -> Result<()> {
    let fcs = FileComponentUrlSource {
        url: "https://example.com/wasm.wasm.wasm".to_owned(),
        digest: "sha256:12345".to_owned(),
    };
    let us = UrlSource::new(&fcs)?;
    assert_eq!("https", us.url().scheme());
    assert_eq!("/wasm.wasm.wasm", us.url().path());
    assert_eq!(PathBuf::from("wasm.wasm.wasm"), us.url_relative_path());
    Ok(())
}

#[tokio::test]
async fn url_sources_are_validated() -> Result<()> {
    let fcs1 = FileComponentUrlSource {
        url: "ftp://example.com/wasm.wasm.wasm".to_owned(),
        digest: "sha256:12345".to_owned(),
    };
    UrlSource::new(&fcs1).expect_err("fcs1 should fail on scheme");

    let fcs2 = FileComponentUrlSource {
        url: "SNORKBONGLY".to_owned(),
        digest: "sha256:12345".to_owned(),
    };
    UrlSource::new(&fcs2).expect_err("fcs2 should fail because not a URL");

    let fcs3 = FileComponentUrlSource {
        url: "https://example.com/wasm.wasm.wasm".to_owned(),
        digest: "sha123:12345".to_owned(),
    };
    UrlSource::new(&fcs3).expect_err("fcs3 should fail on digest fmt");

    let fcs4 = FileComponentUrlSource {
        url: "https://example.com/wasm.wasm.wasm".to_owned(),
        digest: "sha256:".to_owned(),
    };
    UrlSource::new(&fcs4).expect_err("fcs4 should fail on empty digest");

    Ok(())
}

#[tokio::test]
async fn test_invalid_manifest() -> Result<()> {
    const MANIFEST: &str = "tests/invalid-manifest.toml";

    let temp_dir = tempfile::tempdir()?;
    let dir = temp_dir.path();
    let app = from_file(MANIFEST, Some(dir)).await;

    let e = app.unwrap_err().to_string();
    assert!(
        e.contains("invalid-manifest.toml"),
        "Expected error to contain the manifest name"
    );

    Ok(())
}

#[test]
fn test_unknown_version_is_rejected() {
    const MANIFEST: &str = include_str!("../../tests/invalid-version.toml");

    let cfg = raw_manifest_from_str(MANIFEST);
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

    let cfg_any: RawAppManifestAnyVersion = raw_manifest_from_str(MANIFEST)?;
    let cfg = cfg_any.into_v1();

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
    let app = from_file(MANIFEST, Some(dir)).await;

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
    let app = from_file(MANIFEST, Some(dir)).await;

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
    let app = from_file(MANIFEST, Some(dir)).await;

    assert!(app.is_err(), "Expected allowed_http_hosts parsing error");

    let e = app.unwrap_err().to_string();
    assert!(
        e.contains("ftp://some-random-api.ml"),
        "Expected allowed_http_hosts parse error to contain `ftp://some-random-api.ml`"
    );
    assert!(
        e.contains("example.com/wib/wob"),
        "Expected allowed_http_hosts parse error to contain `example.com/wib/wob`"
    );

    Ok(())
}
