use anyhow::{anyhow, bail, Context, Result};
use bindle::Id;
use chrono::{DateTime, Utc};
use clap::Parser;
use cloud::client::{Client as CloudClient, ConnectionConfig};
use cloud_openapi::models::ChannelRevisionSelectionStrategy as CloudChannelRevisionSelectionStrategy;
use cloud_openapi::models::TokenInfo;
use hippo::{Client, ConnectionInfo};
use hippo_openapi::models::ChannelRevisionSelectionStrategy;
use semver::BuildMetadata;
use sha2::{Digest, Sha256};
use spin_http::routes::RoutePattern;
use spin_loader::local::config::{RawAppManifest, RawAppManifestAnyVersion};
use spin_loader::local::{assets, config};
use spin_manifest::{HttpTriggerConfiguration, TriggerConfig};
use spin_publish::BindleConnectionInfo;
use tokio::fs;

use std::fs::File;
use std::io::{copy, Write};
use std::path::PathBuf;
use url::Url;
use uuid::Uuid;

use crate::{opts::*, parse_buildinfo, sloth::warn_if_slow_response};

use super::login::LoginConnection;

const SPIN_DEPLOY_CHANNEL_NAME: &str = "spin-deploy";

const BINDLE_REGISTRY_URL_PATH: &str = "api/registry";

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

    /// Path to assemble the bindle before pushing (defaults to
    /// a temporary directory)
    #[clap(
        name = STAGING_DIR_OPT,
        long = "staging-dir",
        short = 'd',
    )]
    pub staging_dir: Option<PathBuf>,

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

    /// How long in seconds to wait for a deployed HTTP application to become
    /// ready. The default is 60 seconds. Set it to 0 to skip waiting
    /// for readiness.
    #[clap(long = "readiness-timeout", default_value = "60")]
    pub readiness_timeout_secs: u16,
}

impl DeployCommand {
    pub async fn run(self) -> Result<()> {
        let path = dirs::config_dir()
            .context("Cannot find config directory")?
            .join("spin")
            .join("config.json");

        // TODO: invoke LoginCommand::run() if the file cannot be found (not logged in)
        let data = fs::read_to_string(path.clone()).await.context(format!(
            "Cannot find spin config at {}",
            path.to_string_lossy()
        ))?;
        let login_connection: LoginConnection = serde_json::from_str(&data)?;

        // ... or if the token has expired.
        let expiration_date = DateTime::parse_from_rfc3339(&login_connection.expiration)?;
        let now: DateTime<Utc> = Utc::now();
        if now > expiration_date {
            bail!("Your session has expired. Please log back in.")
        }

        check_healthz(&login_connection.url).await?;
        let _sloth_warning = warn_if_slow_response(&login_connection.url);

        // TODO: we should have a smarter check in place here to determine the difference between Hippo and the Cloud APIs
        if login_connection.bindle_url.is_some() {
            self.deploy_hippo(login_connection).await
        } else {
            self.deploy_cloud(login_connection).await
        }
    }

    async fn deploy_hippo(self, login_connection: LoginConnection) -> Result<()> {
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

        let bindle_connection_info = BindleConnectionInfo::new(
            login_connection.bindle_url.unwrap(),
            login_connection.danger_accept_invalid_certs,
            login_connection.bindle_username,
            login_connection.bindle_password,
        );

        let bindle_id = self
            .create_and_push_bindle(buildinfo, bindle_connection_info)
            .await?;

        let hippo_client = Client::new(ConnectionInfo {
            url: login_connection.url.clone(),
            danger_accept_invalid_certs: login_connection.danger_accept_invalid_certs,
            api_key: Some(login_connection.token),
        });

        let name = bindle_id.name().to_string();

        // Create or update app
        // TODO: this process involves many calls to Hippo. Should be able to update the channel
        // via only `add_revision` if bindle naming schema is updated so bindles can be deterministically ordered by Hippo.
        let channel_id = match self.get_app_id_hippo(&hippo_client, name.clone()).await {
            Ok(app_id) => {
                Client::add_revision(
                    &hippo_client,
                    name.clone(),
                    bindle_id.version_string().clone(),
                )
                .await?;
                let existing_channel_id = self
                    .get_channel_id_hippo(
                        &hippo_client,
                        SPIN_DEPLOY_CHANNEL_NAME.to_string(),
                        app_id,
                    )
                    .await?;
                let active_revision_id = self
                    .get_revision_id_hippo(
                        &hippo_client,
                        bindle_id.version_string().clone(),
                        app_id,
                    )
                    .await?;
                Client::patch_channel(
                    &hippo_client,
                    existing_channel_id,
                    None,
                    None,
                    Some(ChannelRevisionSelectionStrategy::UseSpecifiedRevision),
                    None,
                    Some(active_revision_id),
                    None,
                    None,
                )
                .await
                .context("Problem patching a channel in Hippo")?;

                existing_channel_id
            }
            Err(_) => {
                let range_rule = Some(bindle_id.version_string());
                let app_id = Client::add_app(&hippo_client, name.clone(), name.clone())
                    .await
                    .context("Unable to create Hippo app")?;
                Client::add_channel(
                    &hippo_client,
                    app_id,
                    String::from(SPIN_DEPLOY_CHANNEL_NAME),
                    None,
                    ChannelRevisionSelectionStrategy::UseRangeRule,
                    range_rule,
                    None,
                    None,
                )
                .await
                .context("Problem creating a channel in Hippo")?
            }
        };

        // Hippo has responded - we don't want to keep the sloth timer running.
        drop(sloth_warning);

        println!(
            "Deployed {} version {}",
            name.clone(),
            bindle_id.version_string()
        );
        let channel = Client::get_channel_by_id(&hippo_client, &channel_id.to_string())
            .await
            .context("Problem getting channel by id")?;
        if let Ok(http_config) = HttpTriggerConfiguration::try_from(cfg.info.trigger.clone()) {
            wait_for_ready(
                &channel.domain,
                &self.hippo_server_url,
                &cfg,
                self.readiness_timeout_secs,
            )
            .await;
            print_available_routes(
                &channel.domain,
                &http_config.base,
                &login_connection.url,
                &cfg,
            );
        } else {
            println!("Application is running at {}", channel.domain);
        }

        Ok(())
    }

    async fn deploy_cloud(self, login_connection: LoginConnection) -> Result<()> {
        let connection_config = ConnectionConfig {
            url: login_connection.url.clone(),
            insecure: login_connection.danger_accept_invalid_certs,
            token: TokenInfo {
                token: Some(login_connection.token.clone()),
                expiration: Some(login_connection.expiration.clone()),
            },
        };

        let client = CloudClient::new(connection_config.clone());

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

        let bindle_connection_info = BindleConnectionInfo::from_token(
            format!("{}/{}", login_connection.url, BINDLE_REGISTRY_URL_PATH),
            login_connection.danger_accept_invalid_certs,
            login_connection.token,
        );

        let bindle_id = self
            .create_and_push_bindle(buildinfo, bindle_connection_info)
            .await?;
        let name = bindle_id.name().to_string();

        // Create or update app
        // TODO: this process involves many calls to Hippo. Should be able to update the channel
        // via only `add_revision` if bindle naming schema is updated so bindles can be deterministically ordered by Hippo.
        let channel_id = match self.get_app_id_cloud(&client, name.clone()).await {
            Ok(app_id) => {
                CloudClient::add_revision(
                    &client,
                    name.clone(),
                    bindle_id.version_string().clone(),
                )
                .await?;
                let existing_channel_id = self
                    .get_channel_id_cloud(&client, SPIN_DEPLOY_CHANNEL_NAME.to_string(), app_id)
                    .await?;
                let active_revision_id = self
                    .get_revision_id_cloud(&client, bindle_id.version_string().clone(), app_id)
                    .await?;
                CloudClient::patch_channel(
                    &client,
                    existing_channel_id,
                    None,
                    None,
                    Some(CloudChannelRevisionSelectionStrategy::UseSpecifiedRevision),
                    None,
                    Some(active_revision_id),
                    None,
                    None,
                )
                .await
                .context("Problem patching a channel")?;

                existing_channel_id
            }
            Err(_) => {
                let range_rule = Some(bindle_id.version_string());
                let app_id = CloudClient::add_app(&client, &name, &name)
                    .await
                    .context("Unable to create app")?;
                CloudClient::add_channel(
                    &client,
                    app_id,
                    String::from(SPIN_DEPLOY_CHANNEL_NAME),
                    None,
                    CloudChannelRevisionSelectionStrategy::UseRangeRule,
                    range_rule,
                    None,
                    None,
                )
                .await
                .context("Problem creating a channel")?
            }
        };

        println!(
            "Deployed {} version {}",
            name.clone(),
            bindle_id.version_string()
        );
        let channel = CloudClient::get_channel_by_id(&client, &channel_id.to_string())
            .await
            .context("Problem getting channel by id")?;
        if let Ok(http_config) = HttpTriggerConfiguration::try_from(cfg.info.trigger.clone()) {
            print_available_routes(
                &channel.domain,
                &http_config.base,
                &login_connection.url,
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
                let exclude_files = x.wasm.exclude_files.clone().unwrap_or_default();
                let fm = assets::collect(files, &exclude_files, &source_dir)?;
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

    async fn get_app_id_hippo(&self, hippo_client: &Client, name: String) -> Result<Uuid> {
        let apps_vm = Client::list_apps(hippo_client).await?;
        let app = apps_vm.items.iter().find(|&x| x.name == name.clone());
        match app {
            Some(a) => Ok(a.id),
            None => bail!("No app with name: {}", name),
        }
    }

    async fn get_app_id_cloud(&self, cloud_client: &CloudClient, name: String) -> Result<Uuid> {
        let apps_vm = CloudClient::list_apps(cloud_client).await?;
        let app = apps_vm.items.iter().find(|&x| x.name == name.clone());
        match app {
            Some(a) => Ok(a.id),
            None => bail!("No app with name: {}", name),
        }
    }

    async fn get_revision_id_hippo(
        &self,
        hippo_client: &Client,
        bindle_version: String,
        app_id: Uuid,
    ) -> Result<Uuid> {
        let revisions = Client::list_revisions(hippo_client).await?;
        let revision = revisions
            .items
            .iter()
            .find(|&x| x.revision_number == bindle_version && x.app_id == app_id);
        Ok(revision
            .ok_or_else(|| {
                anyhow!(
                    "No revision with version {} and app id {}",
                    bindle_version,
                    app_id
                )
            })?
            .id)
    }

    async fn get_revision_id_cloud(
        &self,
        cloud_client: &CloudClient,
        bindle_version: String,
        app_id: Uuid,
    ) -> Result<Uuid> {
        let revisions = CloudClient::list_revisions(cloud_client).await?;
        let revision = revisions
            .items
            .iter()
            .find(|&x| x.revision_number == bindle_version && x.app_id == app_id);
        Ok(revision
            .ok_or_else(|| {
                anyhow!(
                    "No revision with version {} and app id {}",
                    bindle_version,
                    app_id
                )
            })?
            .id)
    }

    async fn get_channel_id_hippo(
        &self,
        hippo_client: &Client,
        name: String,
        app_id: Uuid,
    ) -> Result<Uuid> {
        let channels_vm = Client::list_channels(hippo_client).await?;
        let channel = channels_vm
            .items
            .iter()
            .find(|&x| x.app_id == app_id && x.name == name.clone());
        match channel {
            Some(c) => Ok(c.id),
            None => bail!("No channel with app_id {} and name {}", app_id, name),
        }
    }

    async fn get_channel_id_cloud(
        &self,
        cloud_client: &CloudClient,
        name: String,
        app_id: Uuid,
    ) -> Result<Uuid> {
        let channels_vm = CloudClient::list_channels(cloud_client).await?;
        let channel = channels_vm
            .items
            .iter()
            .find(|&x| x.app_id == app_id && x.name == name.clone());
        match channel {
            Some(c) => Ok(c.id),
            None => bail!("No channel with app_id {} and name {}", app_id, name),
        }
    }

    async fn create_and_push_bindle(
        &self,
        buildinfo: Option<BuildMetadata>,
        bindle_connection_info: BindleConnectionInfo,
    ) -> Result<Id> {
        let source_dir = crate::app_dir(&self.app)?;

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

        let _sloth_warning = warn_if_slow_response(bindle_connection_info.base_url());

        let publish_result =
            spin_publish::push_all(&dest_dir, bindle_id, bindle_connection_info.clone()).await;

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
                        bindle_id,
                        bindle_connection_info.base_url()
                    )
                });
            }
        }

        Ok(bindle_id.clone())
    }
}

async fn check_healthz(url: &str) -> Result<()> {
    let base_url = url::Url::parse(url)?;
    let healthz_url = base_url.join("/healthz")?;
    reqwest::get(healthz_url.to_string())
        .await?
        .error_for_status()
        .with_context(|| format!("Server {} is unhealthy", base_url))?;
    Ok(())
}

const READINESS_POLL_INTERVAL_SECS: u64 = 2;

async fn wait_for_ready(
    app_domain: &str,
    hippo_url: &str,
    cfg: &spin_loader::local::config::RawAppManifest,
    readiness_timeout_secs: u16,
) {
    if readiness_timeout_secs == 0 {
        return;
    }

    if cfg.components.is_empty() {
        return;
    }

    let url_result = Url::parse(hippo_url);
    let scheme = match &url_result {
        Ok(url) => url.scheme(),
        Err(_) => "http",
    };

    let route = "/healthz";
    let healthz_url = format!("{}://{}{}", scheme, app_domain, route);

    let start = std::time::Instant::now();
    let readiness_timeout = std::time::Duration::from_secs(u64::from(readiness_timeout_secs));
    let poll_interval = tokio::time::Duration::from_secs(READINESS_POLL_INTERVAL_SECS);

    print!("Waiting for application to become ready");
    std::io::stdout().flush().unwrap_or_default();
    loop {
        if is_ready(&healthz_url).await {
            println!("... ready");
            return;
        }

        print!(".");
        std::io::stdout().flush().unwrap_or_default();

        if start.elapsed() >= readiness_timeout {
            println!();
            println!("Application deployed, but Spin could not establish readiness");
            return;
        }
        tokio::time::sleep(poll_interval).await;
    }
}

async fn is_ready(healthz_url: &str) -> bool {
    let resp = reqwest::get(healthz_url).await;
    let (msg, ready) = match resp {
        Err(e) => (format!("error {}", e), false),
        Ok(r) => {
            let status = r.status();
            let ok = status.is_success();
            let desc = if ok { "ready" } else { "not ready" };
            (format!("{desc}, code {status}"), ok)
        }
    };

    tracing::debug!("Polled {} for readiness: {}", healthz_url, msg);
    ready
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
