use crate::controller::Controller;
use crate::metadata_extractor::AppMetadata;
use crate::spin;
use crate::utils;
use anyhow::{Context, Result};
use std::fs;
use tokio::task;

/// Represents a testcase
pub struct TestCase {
    /// name of the testcase
    pub name: String,

    /// name of the app under test
    pub appname: Option<String>,

    /// optional
    /// template to use to create new app
    pub template: Option<String>,

    /// optional
    /// template install args. appended to `spin install templates <template_install_args>
    /// defaults to `--git https://github.com/fermyon/spin`
    pub template_install_args: Option<Vec<String>>,

    /// optional
    /// plugins required for the testcase. e.g. js2wasm for js/ts tests
    pub plugins: Option<Vec<String>>,

    /// optional
    /// if provided, appended to `spin deploy` command
    pub deploy_args: Option<Vec<String>>,

    /// optional
    /// if provided, executed as command line before running `spin build`
    /// useful if some external script or process needs to run before `spin build`
    /// e.g. `npm install` before running `spin build` for `js/ts` tests
    pub pre_build_hooks: Option<Vec<Vec<String>>>,

    /// assertions to run once the app is running
    pub assertions: fn(app: &AppMetadata) -> Result<()>,
}

impl TestCase {
    pub async fn run(&self, controller: &dyn Controller) -> Result<()> {
        controller.name();

        // install spin plugins if requested in testcase config
        if let Some(plugins) = &self.plugins {
            controller
                .install_plugins(plugins.iter().map(|s| s as &str).collect())
                .context("installing plugins")?;
        }

        let appname = match &self.appname {
            Some(appname) => appname.to_owned(),
            None => format!("{}-generated", self.template.as_ref().unwrap()),
        };

        let appdir = spin::appdir(appname.as_str());

        // cleanup existing dir for testcase project code. cleaned up only if testcase is a template based test
        if self.template.is_some() {
            // install spin templates from git repo. if template_install_args is provided uses that
            // defaults to spin repo
            let template_install_args = match &self.template_install_args {
                Some(args) => args.iter().map(|s| s as &str).collect(),
                None => vec!["--git", "https://github.com/fermyon/spin"],
            };

            controller
                .template_install(template_install_args)
                .context("installing templates")?;

            if fs::remove_dir_all(&appdir).is_err() {};

            controller
                .new_app(self.template.as_ref().unwrap(), &appname)
                .context("creating new app")?;
        }

        // run pre-build-steps. It is useful for running any steps required before running `spin build`.
        // e.g. for js/ts tests, we need to run `npm install` before running `spin build`
        if let Some(pre_build_hooks) = &self.pre_build_hooks {
            for pre_build_hook in pre_build_hooks {
                utils::run(pre_build_hook.to_vec(), Some(appdir.to_string()), None)?;
            }
        }

        // run spin build
        controller.build_app(&appname).context("building app")?;

        // run `spin up` (or `spin deploy` for cloud).
        // `AppInstance` has some basic info about the running app like base url, routes (only for cloud) etc.
        let app = controller.run_app(&appname).await.context("running app")?;

        // run test specific assertions
        let metadata = app.metadata.clone();
        let assert_fn = self.assertions;

        let result = task::spawn_blocking(move || assert_fn(&metadata))
            .await
            .context("running testcase specific assertions")
            .unwrap();

        match controller
            .stop_app(Some(app.metadata.clone().name.as_str()), None)
            .await
        {
            Ok(_) => (),
            Err(e) => println!(
                "warn: failed to stop app {} with error {:?}",
                app.metadata.clone().name.as_str(),
                e
            ),
        }

        result
    }
}
