use anyhow::ensure;
use anyhow::{anyhow, bail, Context, Result};
use bindle::Id;
use chrono::{DateTime, Utc};
use clap::Parser;
use cloud::client::{Client as CloudClient, ConnectionConfig};
use cloud_openapi::models::ChannelRevisionSelectionStrategy as CloudChannelRevisionSelectionStrategy;
use hippo::{Client, ConnectionInfo};
use hippo_openapi::models::ChannelRevisionSelectionStrategy;
use rand::Rng;
use semver::BuildMetadata;
use sha2::{Digest, Sha256};
use spin_common::{arg_parser::parse_kv, sloth};
use spin_http::routes::RoutePattern;
use spin_loader::bindle::BindleConnectionInfo;
use spin_loader::local::config::RawAppManifest;
use spin_loader::local::{assets, config, parent_dir};
use spin_manifest::ApplicationTrigger;
use spin_manifest::{HttpTriggerConfiguration, TriggerConfig};
use spin_trigger_http::AppInfo;
use tokio::fs;
use tracing::instrument;

use std::fs::File;
use std::io;
use std::io::{copy, Write};
use std::path::PathBuf;
use url::Url;
use uuid::Uuid;

use crate::{opts::*, parse_buildinfo};

use super::login::LoginCommand;
use super::login::LoginConnection;

const SPIN_DEPLOY_CHANNEL_NAME: &str = "spin-deploy";
const SPIN_DEFAULT_KV_STORE: &str = "default";
const BINDLE_REGISTRY_URL_PATH: &str = "api/registry";

/// Package and upload an application to the Fermyon Platform.
#[derive(Parser, Debug)]
#[clap(about = "Package and upload an application to the Fermyon Platform")]
pub struct DeployCommand {
    /// The application to deploy. This may be a manifest (spin.toml) file, or a
    /// directory containing a spin.toml file.
    /// If omitted, it defaults to "spin.toml".
    #[clap(
        name = APP_MANIFEST_FILE_OPT,
        short = 'f',
        long = "from",
        alias = "file",
        default_value = DEFAULT_MANIFEST_FILE
    )]
    pub app_source: PathBuf,

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

    /// Deploy to the Fermyon instance saved under the specified name.
    /// If omitted, Spin deploys to the default unnamed instance.
    #[clap(
        name = "environment-name",
        long = "environment-name",
        env = DEPLOYMENT_ENV_NAME_ENV
    )]
    pub deployment_env_id: Option<String>,

    /// Set a key/value pair (key=value) in the deployed application's
    /// default store. Any existing value will be overwritten.
    /// Can be used multiple times.
    #[clap(long = "key-value", parse(try_from_str = parse_kv))]
    pub key_values: Vec<(String, String)>,
}

impl DeployCommand {
    pub async fn run(self) -> Result<()> {
        let path = self.config_file_path()?;

        // log in if config.json does not exist or cannot be read
        let data = match fs::read_to_string(path.clone()).await {
            Ok(d) => d,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                match self.deployment_env_id {
                    Some(name) => {
                        // TODO: allow auto redirect to login preserving the name
                        eprintln!("You have no instance saved as '{}'", name);
                        eprintln!("Run `spin login --environment-name {}` to log in", name);
                        std::process::exit(1);
                    }
                    None => {
                        // log in, then read config
                        // TODO: propagate deployment id (or bail if nondefault?)
                        LoginCommand::parse_from(vec!["login"]).run().await?;
                        fs::read_to_string(path.clone()).await?
                    }
                }
            }
            Err(e) => {
                bail!("Could not log in: {}", e);
            }
        };

        let mut login_connection: LoginConnection = serde_json::from_str(&data)?;
        let expired = match has_expired(&login_connection) {
            Ok(val) => val,
            Err(err) => {
                eprintln!("{}\n", err);
                eprintln!("Run `spin login` to log in again");
                std::process::exit(1);
            }
        };

        if expired {
            // if we have a refresh token available, let's try to refresh the token
            match login_connection.refresh_token {
                Some(refresh_token) => {
                    // Only Cloud has support for refresh tokens
                    let connection_config = ConnectionConfig {
                        url: login_connection.url.to_string(),
                        insecure: login_connection.danger_accept_invalid_certs,
                        token: login_connection.token.clone(),
                    };
                    let client = CloudClient::new(connection_config.clone());

                    match client
                        .refresh_token(login_connection.token, refresh_token)
                        .await
                    {
                        Ok(token_info) => {
                            login_connection.token = token_info.token;
                            login_connection.refresh_token = Some(token_info.refresh_token);
                            login_connection.expiration = Some(token_info.expiration);
                            // save new token info
                            let path = self.config_file_path()?;
                            std::fs::write(path, serde_json::to_string_pretty(&login_connection)?)?;
                        }
                        Err(e) => {
                            eprintln!("Failed to refresh token: {}", e);
                            match self.deployment_env_id {
                                Some(name) => {
                                    eprintln!(
                                        "Run `spin login --environment-name {}` to log in again",
                                        name
                                    );
                                }
                                None => {
                                    eprintln!("Run `spin login` to log in again");
                                }
                            }
                            std::process::exit(1);
                        }
                    }
                }
                None => {
                    // session has expired and we have no way to refresh the token - log back in
                    match self.deployment_env_id {
                        Some(name) => {
                            // TODO: allow auto redirect to login preserving the name
                            eprintln!("Your login to this environment has expired");
                            eprintln!(
                                "Run `spin login --environment-name {}` to log in again",
                                name
                            );
                            std::process::exit(1);
                        }
                        None => {
                            LoginCommand::parse_from(vec!["login"]).run().await?;
                            let new_data = fs::read_to_string(path.clone()).await.context(
                                format!("Cannot find spin config at {}", path.to_string_lossy()),
                            )?;
                            login_connection = serde_json::from_str(&new_data)?;
                        }
                    }
                }
            }
        }

        let sloth_guard = sloth::warn_if_slothful(
            2500,
            format!("Checking status ({})\n", login_connection.url),
        );
        check_healthz(&login_connection.url).await?;
        // Server has responded - we don't want to keep the sloth timer running.
        drop(sloth_guard);

        // TODO: we should have a smarter check in place here to determine the difference between Hippo and the Cloud APIs
        if login_connection.bindle_url.is_some() {
            self.deploy_hippo(login_connection).await
        } else {
            const DEVELOPER_CLOUD_FAQ: &str = "https://developer.fermyon.com/cloud/faq";

            self.deploy_cloud(login_connection)
                .await
                .map_err(|e| anyhow!("{:?}\n\nLearn more at {}", e, DEVELOPER_CLOUD_FAQ))
        }
    }

    fn app(&self) -> anyhow::Result<PathBuf> {
        crate::manifest::resolve_file_path(&self.app_source)
    }

    // TODO: unify with login
    fn config_file_path(&self) -> Result<PathBuf> {
        let root = dirs::config_dir()
            .context("Cannot find configuration directory")?
            .join("fermyon");

        let file_stem = match &self.deployment_env_id {
            None => "config",
            Some(id) => id,
        };
        let file = format!("{}.json", file_stem);

        let path = root.join(file);

        Ok(path)
    }

    async fn deploy_hippo(self, login_connection: LoginConnection) -> Result<()> {
        let cfg_any = spin_loader::local::raw_manifest_from_file(&self.app()?).await?;
        let cfg = cfg_any.into_v1();

        ensure!(!cfg.components.is_empty(), "No components in spin.toml!");

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
            url: login_connection.url.to_string(),
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

        println!(
            "Deployed {} version {}",
            name.clone(),
            bindle_id.version_string()
        );
        let channel = Client::get_channel_by_id(&hippo_client, &channel_id.to_string())
            .await
            .context("Problem getting channel by id")?;
        let app_base_url = build_app_base_url(&channel.domain, &login_connection.url)?;
        if let Ok(http_config) = HttpTriggerConfiguration::try_from(cfg.info.trigger.clone()) {
            wait_for_ready(
                &app_base_url,
                &bindle_id.version_string(),
                self.readiness_timeout_secs,
                Destination::Platform,
            )
            .await;
            print_available_routes(&app_base_url, &http_config.base, &cfg);
        } else {
            println!("Application is running at {}", channel.domain);
        }

        Ok(())
    }

    async fn deploy_cloud(self, login_connection: LoginConnection) -> Result<()> {
        let connection_config = ConnectionConfig {
            url: login_connection.url.to_string(),
            insecure: login_connection.danger_accept_invalid_certs,
            token: login_connection.token.clone(),
        };

        let client = CloudClient::new(connection_config.clone());

        let cfg_any = spin_loader::local::raw_manifest_from_file(&self.app()?).await?;
        let cfg = cfg_any.into_v1();

        validate_cloud_app(&cfg)?;

        match cfg.info.trigger {
            ApplicationTrigger::Http(_) => {}
            ApplicationTrigger::Redis(_) => bail!("Redis triggers are not supported"),
            ApplicationTrigger::External(_) => bail!("External triggers are not supported"),
        }

        let buildinfo = if !self.no_buildinfo {
            match &self.buildinfo {
                Some(i) => Some(i.clone()),
                // FIXME(lann): As a workaround for buggy partial bindle uploads,
                // force a new bindle version on every upload.
                None => Some(random_buildinfo()),
            }
        } else {
            None
        };

        let su = Url::parse(login_connection.url.as_str())?;
        let bindle_connection_info = BindleConnectionInfo::from_token(
            su.join(BINDLE_REGISTRY_URL_PATH)?.to_string(),
            login_connection.danger_accept_invalid_certs,
            login_connection.token,
        );

        let bindle_id = self
            .create_and_push_bindle(buildinfo, bindle_connection_info)
            .await?;
        let name = bindle_id.name().to_string();

        println!("Deploying...");

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
                    Some(CloudChannelRevisionSelectionStrategy::UseSpecifiedRevision),
                    None,
                    Some(active_revision_id),
                    None,
                )
                .await
                .context("Problem patching a channel")?;

                for kv in self.key_values {
                    CloudClient::add_key_value_pair(
                        &client,
                        app_id,
                        SPIN_DEFAULT_KV_STORE.to_string(),
                        kv.0,
                        kv.1,
                    )
                    .await
                    .context("Problem creating key/value")?;
                }

                existing_channel_id
            }
            Err(_) => {
                let app_id = CloudClient::add_app(&client, &name, &name)
                    .await
                    .context("Unable to create app")?;

                // When creating the new app, InitialRevisionImport command is triggered
                // which automatically imports all revisions from bindle into db
                // therefore we do not need to call add_revision api explicitly here
                let active_revision_id = self
                    .get_revision_id_cloud(&client, bindle_id.version_string().clone(), app_id)
                    .await?;

                let channel_id = CloudClient::add_channel(
                    &client,
                    app_id,
                    String::from(SPIN_DEPLOY_CHANNEL_NAME),
                    CloudChannelRevisionSelectionStrategy::UseSpecifiedRevision,
                    None,
                    Some(active_revision_id),
                )
                .await
                .context("Problem creating a channel")?;

                for kv in self.key_values {
                    CloudClient::add_key_value_pair(
                        &client,
                        app_id,
                        SPIN_DEFAULT_KV_STORE.to_string(),
                        kv.0,
                        kv.1,
                    )
                    .await
                    .context("Problem creating key/value")?;
                }

                channel_id
            }
        };

        let channel = CloudClient::get_channel_by_id(&client, &channel_id.to_string())
            .await
            .context("Problem getting channel by id")?;
        let app_base_url = build_app_base_url(&channel.domain, &login_connection.url)?;
        if let Ok(http_config) = HttpTriggerConfiguration::try_from(cfg.info.trigger.clone()) {
            wait_for_ready(
                &app_base_url,
                &bindle_id.version_string(),
                self.readiness_timeout_secs,
                Destination::Cloud(connection_config.url),
            )
            .await;
            print_available_routes(&app_base_url, &http_config.base, &cfg);
        } else {
            println!("Application is running at {}", channel.domain);
        }

        Ok(())
    }

    async fn compute_buildinfo(&self, cfg: &RawAppManifest) -> Result<BuildMetadata> {
        let app_file = self.app()?;
        let mut sha256 = Sha256::new();
        let app_folder = parent_dir(&app_file)?;

        for x in cfg.components.iter() {
            match &x.source {
                config::RawModuleSource::FileReference(p) => {
                    let full_path = app_folder.join(p);
                    let mut r = File::open(&full_path)
                        .with_context(|| anyhow!("Cannot open file {}", &full_path.display()))?;
                    copy(&mut r, &mut sha256)?;
                }
                config::RawModuleSource::Url(us) => sha256.update(us.digest.as_bytes()),
            }

            if let Some(files) = &x.wasm.files {
                let exclude_files = x.wasm.exclude_files.clone().unwrap_or_default();
                let fm = assets::collect(files, &exclude_files, &app_folder)?;
                for f in fm.iter() {
                    let mut r = File::open(&f.src)
                        .with_context(|| anyhow!("Cannot open file {}", &f.src.display()))?;
                    copy(&mut r, &mut sha256)?;
                }
            }
        }

        let mut r = File::open(&app_file)?;
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
        let mut revisions = cloud_client.list_revisions().await?;

        loop {
            if let Some(revision) = revisions
                .items
                .iter()
                .find(|&x| x.revision_number == bindle_version && x.app_id == app_id)
            {
                return Ok(revision.id);
            }

            if revisions.is_last_page {
                break;
            }

            revisions = cloud_client.list_revisions_next(&revisions).await?;
        }

        Err(anyhow!(
            "No revision with version {} and app id {}",
            bindle_version,
            app_id
        ))
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
        let mut channels_vm = cloud_client.list_channels().await?;

        loop {
            if let Some(channel) = channels_vm
                .items
                .iter()
                .find(|&x| x.app_id == app_id && x.name == name.clone())
            {
                return Ok(channel.id);
            }

            if channels_vm.is_last_page {
                break;
            }

            channels_vm = cloud_client.list_channels_next(&channels_vm).await?;
        }

        Err(anyhow!(
            "No channel with app_id {} and name {}",
            app_id,
            name
        ))
    }

    async fn create_and_push_bindle(
        &self,
        buildinfo: Option<BuildMetadata>,
        bindle_connection_info: BindleConnectionInfo,
    ) -> Result<Id> {
        let temp_dir = tempfile::tempdir()?;
        let dest_dir = match &self.staging_dir {
            None => temp_dir.path(),
            Some(path) => path.as_path(),
        };

        let bindle_id = spin_bindle::prepare_bindle(&self.app()?, buildinfo, dest_dir)
            .await
            .map_err(crate::wrap_prepare_bindle_error)?;

        println!(
            "Uploading {} version {}...",
            bindle_id.name(),
            bindle_id.version()
        );

        match spin_bindle::push_all(dest_dir, &bindle_id, bindle_connection_info.clone()).await {
            Err(spin_bindle::PublishError::BindleAlreadyExists(err_msg)) => {
                if self.redeploy {
                    Ok(bindle_id.clone())
                } else {
                    Err(anyhow!(
                        "Failed to push bindle to server.\n{}\nTry using the --deploy-existing-bindle flag",
                        err_msg
                    ))
                }
            }
            Err(spin_bindle::PublishError::BindleClient(bindle::client::ClientError::Other(e)))
                if e.to_string().contains("application exceeds") =>
            {
                Err(anyhow!(e.trim_start_matches("Unknown error: ").to_owned()))
            }
            Err(err) => Err(err).with_context(|| {
                crate::push_all_failed_msg(dest_dir, bindle_connection_info.base_url())
            }),
            Ok(()) => Ok(bindle_id.clone()),
        }
    }
}

fn validate_cloud_app(app: &RawAppManifest) -> Result<()> {
    ensure!(!app.components.is_empty(), "No components in spin.toml!");
    for component in &app.components {
        if let Some(invalid_store) = component
            .wasm
            .key_value_stores
            .iter()
            .flatten()
            .find(|store| *store != "default")
        {
            bail!("Invalid store {invalid_store:?} for component {:?}. Cloud currently supports only the 'default' store.", component.id);
        }
    }
    Ok(())
}

fn random_buildinfo() -> BuildMetadata {
    let random_bytes: [u8; 4] = rand::thread_rng().gen();
    let random_hex: String = random_bytes.iter().map(|b| format!("{:x}", b)).collect();
    BuildMetadata::new(&format!("r{random_hex}")).unwrap()
}

fn build_app_base_url(app_domain: &str, hippo_url: &Url) -> Result<Url> {
    // HACK: We assume that the scheme (https vs http) of apps will match that of Hippo...
    let scheme = hippo_url.scheme();
    Url::parse(&format!("{scheme}://{app_domain}/")).with_context(|| {
        format!("Could not construct app base URL for {app_domain:?} (Hippo URL: {hippo_url:?})",)
    })
}

async fn check_healthz(base_url: &Url) -> Result<()> {
    let healthz_url = base_url.join("healthz")?;
    reqwest::get(healthz_url)
        .await?
        .error_for_status()
        .with_context(|| format!("Server {} is unhealthy", base_url))?;
    Ok(())
}

const READINESS_POLL_INTERVAL_SECS: u64 = 2;

enum Destination {
    Cloud(String),
    Platform,
}

async fn wait_for_ready(
    app_base_url: &Url,
    bindle_version: &str,
    readiness_timeout_secs: u16,
    destination: Destination,
) {
    if readiness_timeout_secs == 0 {
        return;
    }

    let app_info_url = app_base_url
        .join(spin_http::WELL_KNOWN_PREFIX.trim_start_matches('/'))
        .unwrap()
        .join("info")
        .unwrap()
        .to_string();

    let start = std::time::Instant::now();
    let readiness_timeout = std::time::Duration::from_secs(u64::from(readiness_timeout_secs));
    let poll_interval = tokio::time::Duration::from_secs(READINESS_POLL_INTERVAL_SECS);

    print!("Waiting for application to become ready");
    let _ = std::io::stdout().flush();
    loop {
        match is_ready(&app_info_url, bindle_version).await {
            Err(err) => {
                println!("... readiness check failed: {err:?}");
                return;
            }
            Ok(true) => {
                println!("... ready");
                return;
            }
            Ok(false) => {}
        }

        print!(".");
        let _ = std::io::stdout().flush();

        if start.elapsed() >= readiness_timeout {
            println!();
            println!("Application deployed, but Spin could not establish readiness");
            if let Destination::Cloud(url) = destination {
                println!("Check the Fermyon Cloud dashboard to see the application status: {url}");
            }
            return;
        }
        tokio::time::sleep(poll_interval).await;
    }
}

#[instrument(level = "debug")]
async fn is_ready(app_info_url: &str, expected_version: &str) -> Result<bool> {
    // If the request fails, we assume the app isn't ready
    let resp = match reqwest::get(app_info_url).await {
        Ok(resp) => resp,
        Err(err) => {
            tracing::warn!("Readiness check failed: {err:?}");
            return Ok(false);
        }
    };
    // If the response status isn't success, the app isn't ready
    if !resp.status().is_success() {
        tracing::debug!("App not ready: {}", resp.status());
        return Ok(false);
    }
    // If the app was previously deployed then it will have an outdated bindle
    // version, in which case the app isn't ready
    if let Ok(app_info) = resp.json::<AppInfo>().await {
        let active_version = app_info.bindle_version;
        if active_version.as_deref() != Some(expected_version) {
            tracing::debug!("Active version {active_version:?} != expected {expected_version:?}");
            return Ok(false);
        }
    }
    Ok(true)
}

fn print_available_routes(
    app_base_url: &Url,
    base: &str,
    cfg: &spin_loader::local::config::RawAppManifest,
) {
    if cfg.components.is_empty() {
        return;
    }

    // Strip any trailing slash from base URL
    let app_base_url = app_base_url.to_string();
    let route_prefix = app_base_url.strip_suffix('/').unwrap_or(&app_base_url);

    println!("Available Routes:");
    for component in &cfg.components {
        if let TriggerConfig::Http(http_cfg) = &component.trigger {
            let route = RoutePattern::from(base, &http_cfg.route);
            println!("  {}: {}{}", component.id, route_prefix, route);
            if let Some(description) = &component.description {
                println!("    {}", description);
            }
        }
    }
}

// Check if the token has expired.
// If the expiration is None, assume the token has not expired
fn has_expired(login_connection: &LoginConnection) -> Result<bool> {
    match &login_connection.expiration {
        Some(expiration) => match DateTime::parse_from_rfc3339(expiration) {
            Ok(time) => Ok(Utc::now() > time),
            Err(err) => Err(anyhow!(
                "Failed to parse token expiration time '{}'. Error: {}",
                expiration,
                err
            )),
        },
        None => Ok(false),
    }
}
