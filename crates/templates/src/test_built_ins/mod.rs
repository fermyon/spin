#![cfg(test)]

// Module for unit-testing the built-in templates when a full integration test would be overkill.
// If your test involves invoking the Spin CLI, or builds or runs an application, use
// an integration test.

use std::{collections::HashMap, path::PathBuf};

use super::*;

struct DiscardingReporter;

impl ProgressReporter for DiscardingReporter {
    fn report(&self, _: impl AsRef<str>) {}
}

#[tokio::test]
async fn add_fileserver_does_not_create_dir() -> anyhow::Result<()> {
    let built_ins_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let built_ins_src = TemplateSource::File(built_ins_dir);

    let store_dir = tempfile::tempdir()?;
    let store = store::TemplateStore::new(store_dir.path());
    let manager = TemplateManager::new(store);

    manager
        .install(
            &built_ins_src,
            &InstallOptions::default(),
            &DiscardingReporter,
        )
        .await?;

    let app_dir = tempfile::tempdir()?;

    // Create an app to add the fileserver into
    let new_empty_options = RunOptions {
        variant: TemplateVariantInfo::NewApplication,
        name: "add-fs-dir-test".to_owned(),
        output_path: app_dir.path().to_owned(),
        values: HashMap::new(),
        accept_defaults: true,
        no_vcs: false,
    };
    manager
        .get("http-empty")?
        .expect("http-empty template should exist")
        .run(new_empty_options)
        .silent()
        .await?;

    // Add the fileserver to that app
    let manifest_path = app_dir.path().join("spin.toml");
    let add_fs_options = RunOptions {
        variant: TemplateVariantInfo::AddComponent { manifest_path },
        name: "fs".to_owned(),
        output_path: app_dir.path().join("fs"),
        values: HashMap::new(),
        accept_defaults: true,
        no_vcs: false,
    };
    manager
        .get("static-fileserver")?
        .expect("static-fileserver template should exist")
        .run(add_fs_options)
        .silent()
        .await?;

    // Finally!
    assert!(
        !app_dir.path().join("fs").exists(),
        "<app_dir>/fs should not have been created"
    );
    Ok(())
}
