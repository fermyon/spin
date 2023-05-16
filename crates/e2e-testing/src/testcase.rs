use crate::controller::Controller;
use crate::metadata_extractor::AppMetadata;
use crate::spin;
use crate::utils;
use anyhow::{Context, Result};
use core::pin::Pin;
use derive_builder::Builder;
use std::fs;
use std::future::Future;
use tempfile::TempDir;
use tokio::io::BufReader;
use tokio::process::{ChildStderr, ChildStdout};

type ChecksFunc = Box<
    dyn Fn(
        AppMetadata,
        Option<BufReader<ChildStdout>>,
        Option<BufReader<ChildStderr>>,
    ) -> Pin<Box<dyn Future<Output = Result<()>>>>,
>;

/// Represents a testcase
#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct TestCase {
    /// name of the testcase
    pub name: String,

    /// name of the app under test
    #[builder(default)]
    pub appname: Option<String>,

    /// optional
    /// template to use to create new app
    #[builder(default)]
    pub template: Option<String>,

    /// optional
    /// if provided, appended to `spin new` command
    #[builder(default = "vec![]")]
    pub new_app_args: Vec<String>,

    /// trigger type for this spin app
    #[builder(default = "\"http\".to_string()")]
    pub trigger_type: String,

    /// optional
    /// template install args. appended to `spin install templates <template_install_args>
    /// defaults to `--git https://github.com/fermyon/spin`
    #[builder(default)]
    pub template_install_args: Option<Vec<String>>,

    /// optional
    /// plugins required for the testcase. e.g. js2wasm for js/ts tests
    #[builder(default)]
    pub plugins: Option<Vec<String>>,

    /// optional
    /// if provided, appended to `spin up/deploy` command
    #[builder(default = "vec![]")]
    pub deploy_args: Vec<String>,

    /// optional
    /// if provided, executed as command line before running `spin build`
    /// useful if some external script or process needs to run before `spin build`
    /// e.g. `npm install` before running `spin build` for `js/ts` tests
    #[builder(default)]
    pub pre_build_hooks: Option<Vec<Vec<String>>>,

    /// assertions to run once the app is running
    #[builder(setter(custom))]
    pub assertions: ChecksFunc,

    /// registry app url where app is pushed and run from
    #[builder(default)]
    pub push_to_registry: Option<String>,
}

impl TestCaseBuilder {
    pub fn assertions(
        self,
        value: impl Fn(
                AppMetadata,
                Option<BufReader<ChildStdout>>,
                Option<BufReader<ChildStderr>>,
            ) -> Pin<Box<dyn Future<Output = Result<()>>>>
            + 'static,
    ) -> Self {
        let mut this = self;
        this.assertions = Some(Box::new(value));
        this
    }
}

impl TestCase {
    pub async fn run(&self, controller: &dyn Controller) -> Result<()> {
        // install spin plugins if requested in testcase config
        if let Some(plugins) = &self.plugins {
            controller
                .install_plugins(plugins.iter().map(|s| s as &str).collect())
                .context("installing plugins")?;
        }

        let appname = self
            .appname
            .clone()
            .unwrap_or_else(|| format!("{}-generated", self.template.as_ref().unwrap()));

        let appdir = spin::appdir(&appname);

        // cleanup existing dir for testcase project code. cleaned up only if testcase is a template based test
        if let Some(template) = &self.template {
            // install spin templates from git repo. if template_install_args is provided uses that
            // defaults to spin repo
            let template_install_args = match &self.template_install_args {
                Some(args) => args.iter().map(|s| s as &str).collect(),
                None => vec!["--git", "https://github.com/fermyon/spin"],
            };

            controller
                .template_install(template_install_args)
                .context("installing templates")?;

            let _ = fs::remove_dir_all(&appdir);

            let new_app_args = self.new_app_args.iter().map(|s| s as &str).collect();
            controller
                .new_app(template, &appname, new_app_args)
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

        //push to registry if url provided
        if let Some(registry_app_url) = &self.push_to_registry {
            spin::registry_push(&appname, registry_app_url.as_str())?;
        }

        // create temp dir as state dir
        let tempdir = TempDir::new()?;
        let state_dir: String = tempdir
            .path()
            .join(".spin")
            .into_os_string()
            .into_string()
            .unwrap();

        // run `spin up` (or `spin deploy` for cloud).
        // `AppInstance` has some basic info about the running app like base url, routes (only for cloud) etc.
        let deploy_args = self.deploy_args.iter().map(|s| s as &str).collect();
        let app = controller
            .run_app(
                &appname,
                &self.trigger_type,
                deploy_args,
                state_dir.as_str(),
            )
            .await
            .context("running app")?;

        // run test specific assertions
        let deployed_app_metadata = app.metadata;
        let deployed_app_name = deployed_app_metadata.name.clone();

        let assertions_result =
            (self.assertions)(deployed_app_metadata, app.stdout_stream, app.stderr_stream).await;

        if let Err(e) = controller
            .stop_app(Some(deployed_app_name.as_str()), app.process)
            .await
        {
            println!("warn: failed to stop app {deployed_app_name} with error {e:?}");
        }

        // this ensure tempdir cleans up after running test
        drop(tempdir);

        assertions_result
    }
}
