use crate::controller::{AppInstance, Controller};
use crate::spin;
use crate::utils;
use anyhow::{Context, Result};
use std::fs;
use tokio::task;

pub struct SkipCondition {
    pub env: String,
    pub reason: String,
}

impl SkipCondition {
    pub fn skip(&self, controller: &dyn Controller) -> bool {
        controller.name() == self.env
    }
}

/// Represents a testcase
pub struct TestCase {
    /// name of the testcase
    pub name: String,

    /// name of the app under test
    pub appname: String,

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
    /// conditions where this testcase should be skipped
    /// currently only supports skipping based on controller name
    pub skip_conditions: Option<Vec<SkipCondition>>,

    /// optional
    /// if provided, appended to `spin deploy` command
    pub deploy_args: Option<Vec<String>>,

    /// optional
    /// if provided, executed as command line before running `spin build`
    /// useful if some external script or process needs to run before `spin build`
    /// e.g. `npm install` before running `spin build` for `js/ts` tests
    pub pre_build_hooks: Option<Vec<Vec<String>>>,

    /// assertions to run once the app is running
    pub assertions: fn(app: &AppInstance) -> Result<()>,
}

impl TestCase {
    pub async fn run(&self, controller: &dyn Controller) -> Result<()> {
        controller.name();

        // evaluate the skip conditions specified in testcase config.
        if let Some(skip_conditions) = &self.skip_conditions {
            for skip_condition in skip_conditions {
                if skip_condition.skip(controller) {
                    return Ok(());
                }
            }
        }

        // install spin templates from git repo.
        // if template_install_args is provided uses that
        // defaults to spin repo
        let template_install_args = match &self.template_install_args {
            Some(args) => args.iter().map(|s| s as &str).collect(),
            None => vec!["--git", "https://github.com/fermyon/spin"],
        };

        controller
            .template_install(template_install_args)
            .context("installing templates")?;

        // install spin plugins if requested in testcase config
        if let Some(plugins) = &self.plugins {
            controller
                .install_plugins(plugins.iter().map(|s| s as &str).collect())
                .context("installing plugins")?;
        }

        let appdir = spin::appdir(&self.appname);

        // cleanup existing dir for testcase project code. cleaned up only if testcase is a template based test
        if self.template.is_some() {
            if fs::remove_dir_all(&appdir).is_err() {};

            controller
                .new_app(self.template.as_ref().unwrap(), &self.appname)
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
        controller.build_app(&self.appname).context("builing app")?;

        // run `spin up` (or `spin deploy` for cloud).
        // `AppInstance` has some basic info about the running app like base url, routes (only for cloud) etc.
        let app = controller
            .run_app(&self.appname)
            .await
            .context("deploying app")?;

        // run test specific assertions
        let assert_fn = self.assertions;

        return task::spawn_blocking(move || assert_fn(&app))
            .await
            .context("running testcase specific assertions")?;
    }
}
