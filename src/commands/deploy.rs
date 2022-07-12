use anyhow::{anyhow, Context, Result};
use bindle::Id;
use clap::Parser;
use hippo::{Client, ConnectionInfo};
use hippo_openapi::models::ChannelRevisionSelectionStrategy;
use semver::BuildMetadata;
use sha2::{Digest, Sha256};
use spin_http_engine::routes::RoutePattern;
use spin_loader::local::config::{RawAppManifest, RawAppManifestAnyVersion};
use spin_loader::local::{assets, config};
use spin_manifest::{HttpTriggerConfiguration, TriggerConfig};
use std::fs::File;
use std::io::copy;
use std::path::PathBuf;
use url::Url;

use crate::{opts::*, parse_buildinfo, sloth::warn_if_slow_response};

/// Package and upload Spin artifacts, notifying Hippo
#[derive(Parser, Debug)]
#[clap(about = "Deploy a Spin application")]
pub struct DeployCommand {
    /// Path to spin.toml
    #[clap(
        name = APP_CONFIG_FILE_OPT,
        short = 'f',
        long = "file",
        default_value = "spin.toml"
    )]
    pub app: PathBuf,

    /// URL of bindle server
    #[clap(
        name = BINDLE_SERVER_URL_OPT,
        long = "bindle-server",
        env = BINDLE_URL_ENV,
    )]
    pub bindle_server_url: String,

    /// Basic http auth username for the bindle server
    #[clap(
        name = BINDLE_USERNAME,
        long = "bindle-username",
        env = BINDLE_USERNAME,
        requires = BINDLE_PASSWORD
    )]
    pub bindle_username: Option<String>,

    /// Basic http auth password for the bindle server
    #[clap(
        name = BINDLE_PASSWORD,
        long = "bindle-password",
        env = BINDLE_PASSWORD,
        requires = BINDLE_USERNAME
    )]
    pub bindle_password: Option<String>,

    /// Ignore server certificate errors from bindle and hippo
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// URL of hippo server
    #[clap(
        name = HIPPO_SERVER_URL_OPT,
        long = "hippo-server",
        env = HIPPO_URL_ENV,
    )]
    pub hippo_server_url: String,

    /// Path to assemble the bindle before pushing (defaults to
    /// a temporary directory)
    #[clap(
        name = STAGING_DIR_OPT,
        long = "staging-dir",
        short = 'd',
    )]
    pub staging_dir: Option<PathBuf>,

    /// Hippo username
    #[clap(
        name = "HIPPO_USERNAME",
        long = "hippo-username",
        env = "HIPPO_USERNAME"
    )]
    pub hippo_username: String,

    /// Hippo password
    #[clap(
        name = "HIPPO_PASSWORD",
        long = "hippo-password",
        env = "HIPPO_PASSWORD"
    )]
    pub hippo_password: String,

    /// Disable attaching buildinfo
    #[clap(
        long = "no-buildinfo",
        conflicts_with = BUILDINFO_OPT,
        env = "SPIN_DEPLOY_NO_BUILDINFO"
    )]
    pub no_buildinfo: bool,

    /// Build metadata to append to the bindle version
    #[clap(
        name = BUILDINFO_OPT,
        long = "buildinfo",
        parse(try_from_str = parse_buildinfo),
    )]
    pub buildinfo: Option<BuildMetadata>,

    /// Deploy existing bindle if it already exists on bindle server
    #[clap(short = 'e', long = "deploy-existing-bindle")]
    pub redeploy: bool,
}

impl DeployCommand {
    pub async fn run(self) -> Result<()> {
        let cfg_any = spin_loader::local::raw_manifest_from_file(&self.app).await?;
        let RawAppManifestAnyVersion::V1(cfg) = cfg_any;

        let buildinfo = if !self.no_buildinfo {
            match &self.buildinfo {
                Some(i) => Some(i.clone()),
                None => self.compute_buildinfo(&cfg).await.map(Option::Some)?,
            }
        } else {
            None
        };

        self.check_hippo_healthz().await?;

        let bindle_id = self.create_and_push_bindle(buildinfo).await?;

        let _sloth_warning = warn_if_slow_response(&self.hippo_server_url);

        let token = match Client::login(
            &Client::new(ConnectionInfo {
                url: self.hippo_server_url.clone(),
                danger_accept_invalid_certs: self.insecure,
                api_key: None,
            }),
            self.hippo_username.clone(),
            self.hippo_password.clone(),
        )
        .await?
        .token
        {
            Some(t) => t,
            None => String::from(""),
        };

        let hippo_client = Client::new(ConnectionInfo {
            url: self.hippo_server_url.clone(),
            danger_accept_invalid_certs: self.insecure,
            api_key: Some(token),
        });

        let name = bindle_id.name().to_string();

        // delete app if it exists in Hippo already
        if let Ok(id) = self.get_app_id(&hippo_client, name.clone()).await {
            Client::remove_app(&hippo_client, id)
                .await
                .context("Problem cleaning up existing Hippo app")?
        }

        let app_id = Client::add_app(&hippo_client, name.clone(), name.clone())
            .await
            .context("Unable to create Hippo app")?;

        let channel_id = Client::add_channel(
            &hippo_client,
            app_id,
            String::from("spin-deploy"),
            None,
            ChannelRevisionSelectionStrategy::UseRangeRule,
            Some(bindle_id.version_string()),
            None,
            None,
        )
        .await
        .context("Problem creating a channel in Hippo")?;

        println!(
            "Deployed {} version {}",
            name.clone(),
            bindle_id.version_string()
        );
        let channel = Client::get_channel_by_id(&hippo_client, &channel_id)
            .await
            .context("Problem getting channel by id")?;
        if let Ok(http_config) = HttpTriggerConfiguration::try_from(cfg.info.trigger.clone()) {
            print_available_routes(
                &channel.domain,
                &http_config.base,
                &self.hippo_server_url,
                &cfg,
            );
        } else {
            println!("Application is running at {}", channel.domain);
        }

        Ok(())
    }

    async fn compute_buildinfo(&self, cfg: &RawAppManifest) -> Result<BuildMetadata> {
        let mut sha256 = Sha256::new();
        let app_folder = self.app.parent().with_context(|| {
            anyhow!(
                "Cannot get a parent directory of manifest file {}",
                &self.app.display()
            )
        })?;

        for x in cfg.components.iter() {
            match &x.source {
                config::RawModuleSource::FileReference(p) => {
                    let full_path = app_folder.join(p);
                    let mut r = File::open(&full_path)
                        .with_context(|| anyhow!("Cannot open file {}", &full_path.display()))?;
                    copy(&mut r, &mut sha256)?;
                }
                config::RawModuleSource::Bindle(_b) => {}
            }
            if let Some(files) = &x.wasm.files {
                let source_dir = crate::app_dir(&self.app)?;
                let fm = assets::collect(files, &source_dir)?;
                for f in fm.iter() {
                    let mut r = File::open(&f.src)
                        .with_context(|| anyhow!("Cannot open file {}", &f.src.display()))?;
                    copy(&mut r, &mut sha256)?;
                }
            }
        }

        let mut r = File::open(&self.app)?;
        copy(&mut r, &mut sha256)?;

        let mut final_digest = format!("q{:x}", sha256.finalize());
        final_digest.truncate(8);

        let buildinfo =
            BuildMetadata::new(&final_digest).with_context(|| "Could not compute build info")?;

        Ok(buildinfo)
    }

    async fn get_app_id(&self, hippo_client: &Client, name: String) -> Result<String> {
        let apps_vm = Client::list_apps(hippo_client).await?;
        let app = apps_vm.apps.iter().find(|&x| x.name == name.clone());
        match app {
            Some(a) => Ok(a.id.clone()),
            None => anyhow::bail!("No app with name: {}", name),
        }
    }

    async fn create_and_push_bindle(&self, buildinfo: Option<BuildMetadata>) -> Result<Id> {
        let source_dir = crate::app_dir(&self.app)?;
        let bindle_connection_info = spin_publish::BindleConnectionInfo::new(
            &self.bindle_server_url,
            self.insecure,
            self.bindle_username.clone(),
            self.bindle_password.clone(),
        );

        let temp_dir = tempfile::tempdir()?;
        let dest_dir = match &self.staging_dir {
            None => temp_dir.path(),
            Some(path) => path.as_path(),
        };
        let (invoice, sources) = spin_publish::expand_manifest(&self.app, buildinfo, &dest_dir)
            .await
            .with_context(|| format!("Failed to expand '{}' to a bindle", self.app.display()))?;

        let bindle_id = &invoice.bindle.id;

        spin_publish::write(&source_dir, &dest_dir, &invoice, &sources)
            .await
            .with_context(|| crate::write_failed_msg(bindle_id, dest_dir))?;

        let _sloth_warning = warn_if_slow_response(&self.bindle_server_url);

        let publish_result =
            spin_publish::push_all(&dest_dir, bindle_id, bindle_connection_info).await;

        if let Err(publish_err) = publish_result {
            // TODO: maybe use `thiserror` to return type errors.
            let already_exists = publish_err
                .to_string()
                .contains("already exists on the server");
            if already_exists {
                if self.redeploy {
                    return Ok(bindle_id.clone());
                } else {
                    return Err(anyhow!(
                        "Failed to push bindle to server.\n{}\nTry using the --deploy-existing-bindle flag",
                        publish_err
                    ));
                }
            } else {
                return Err(publish_err).with_context(|| {
                    format!(
                        "Failed to push bindle {} to server {}",
                        bindle_id, self.bindle_server_url
                    )
                });
            }
        }

        Ok(bindle_id.clone())
    }

    async fn check_hippo_healthz(&self) -> Result<()> {
        let hippo_base_url = url::Url::parse(&self.hippo_server_url)?;
        let hippo_healthz_url = hippo_base_url.join("/healthz")?;
        let result = reqwest::get(hippo_healthz_url.to_string())
            .await?
            .error_for_status()?
            .text()
            .await?;
        if result != "Healthy" {
            return Err(anyhow!("Hippo server {} is unhealthy", hippo_base_url));
        }
        Ok(())
    }
}

fn print_available_routes(
    address: &str,
    base: &str,
    hippo_url: &str,
    cfg: &spin_loader::local::config::RawAppManifest,
) {
    if cfg.components.is_empty() {
        return;
    }

    println!("Available Routes:");
    for component in &cfg.components {
        if let TriggerConfig::Http(http_cfg) = &component.trigger {
            let url_result = Url::parse(hippo_url);
            let scheme = match &url_result {
                Ok(url) => url.scheme(),
                Err(_) => "http",
            };

            let route = RoutePattern::from(base, &http_cfg.route);
            println!("  {}: {}://{}{}", component.id, scheme, address, route);
            if let Some(description) = &component.description {
                println!("    {}", description);
            }
        }
    }
}
