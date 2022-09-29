use anyhow::{anyhow, bail, Context, Result};
use bindle::Id;
use clap::Parser;
use copypasta::{ClipboardContext, ClipboardProvider};
use cloud::client::Client as CloudClient;
use cloud::config::ConnectionConfig;
use hippo::{Client, ConnectionInfo};
use hippo_openapi::models::ChannelRevisionSelectionStrategy;
use semver::BuildMetadata;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use spin_http::routes::RoutePattern;
use spin_loader::local::config::{RawAppManifest, RawAppManifestAnyVersion};
use spin_loader::local::{assets, config};
use spin_manifest::{HttpTriggerConfiguration, TriggerConfig};

use std::fs::File;
use std::io::{copy, Write};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

use crate::{opts::*, parse_buildinfo, sloth::warn_if_slow_response};

const SPIN_DEPLOY_CHANNEL_NAME: &str = "spin-deploy";

// this is the client ID registered in the Cloud's backend
const SPIN_CLIENT_ID: &str = "583e63e9-461f-4fbe-a246-23e0fb1cad10";

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
        requires = HIPPO_USERNAME,
        requires = HIPPO_PASSWORD,
    )]
    pub bindle_server_url: Option<String>,

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
        name = HIPPO_USERNAME,
        long = "hippo-username",
        env = HIPPO_USERNAME,
        requires = BINDLE_SERVER_URL_OPT,
        requires = HIPPO_PASSWORD,
    )]
    pub hippo_username: Option<String>,

    /// Hippo password
    #[clap(
        name = HIPPO_PASSWORD,
        long = "hippo-password",
        env = HIPPO_PASSWORD,
        requires = BINDLE_SERVER_URL_OPT,
        requires = HIPPO_USERNAME,
    )]
    pub hippo_password: Option<String>,

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
        if self.hippo_username.is_some() {
            self.deploy_hippo().await
        } else {
            self.deploy_cloud().await
        }
    }

    async fn deploy_hippo(self) -> Result<()> {
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

        let sloth_warning = warn_if_slow_response(&self.hippo_server_url);

        let token = match Client::login(
            &Client::new(ConnectionInfo {
                url: self.hippo_server_url.clone(),
                danger_accept_invalid_certs: self.insecure,
                api_key: None,
            }),
            self.hippo_username.as_deref().unwrap().to_string(),
            self.hippo_password.as_deref().unwrap().to_string(),
        )
        .await
        {
            Ok(token_info) => token_info.token.unwrap_or_default(),
            Err(err) => bail!(format_login_error(&err)?),
        };

        let hippo_client = Client::new(ConnectionInfo {
            url: self.hippo_server_url.clone(),
            danger_accept_invalid_certs: self.insecure,
            api_key: Some(token),
        });

        let name = bindle_id.name().to_string();

        // Create or update app
        // TODO: this process involves many calls to Hippo. Should be able to update the channel
        // via only `add_revision` if bindle naming schema is updated so bindles can be deterministically ordered by Hippo.
        let channel_id = match self.get_app_id(&hippo_client, name.clone()).await {
            Ok(app_id) => {
                Client::add_revision(
                    &hippo_client,
                    name.clone(),
                    bindle_id.version_string().clone(),
                )
                .await?;
                let existing_channel_id = self
                    .get_channel_id(&hippo_client, SPIN_DEPLOY_CHANNEL_NAME.to_string(), app_id)
                    .await?;
                let active_revision_id = self
                    .get_revision_id(&hippo_client, bindle_id.version_string().clone(), app_id)
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
                &self.hippo_server_url,
                &cfg,
            );
        } else {
            println!("Application is running at {}", channel.domain);
        }

        Ok(())
    }

    async fn deploy_cloud(self) -> Result<()> {
        let mut connection_config = ConnectionConfig {
            url: self.hippo_server_url.clone(),
            insecure: self.insecure,
            token: Default::default(),
        };

        connection_config.token = self.github_token(connection_config.clone()).await?;

        let client = CloudClient::new(connection_config.clone());

        client
            .create_application(None, self.app, self.buildinfo, connection_config)
            .await?;

        Ok(())
    }

    async fn github_token(
        &self,
        connection_config: ConnectionConfig,
    ) -> Result<cloud_openapi::models::TokenInfo> {
        let client = CloudClient::new(connection_config);

        // Generate a device code and a user code to activate it with
        let device_code = client
            .create_device_code(Uuid::parse_str(SPIN_CLIENT_ID)?)
            .await?;

        // Copy the user code to the clipboard.

        // TODO(radu): should this interact with a user's clipboard?
        // This was added purely for convenience, particularly because the token
        // returned by our Platform is short lived, which means a user would have to
        // perform the login process every 30 minutes by default, which sounds
        // VERY aggressive.

        // TODO(radu): this works on macOS, but might fail on other systems.
        // Also, there should be a way to disable it.

        // This works on Linux, but needs an extra library installed, which is not very easy to find.
        let user_code = device_code.user_code.clone().unwrap();
        let copied_to_clipboard = try_copy_to_clipboard(&user_code);

        println!(
            "Open the Cloud's device authorization URL in your browser: {} and enter the code: {}",
            device_code.verification_url.clone().unwrap(),
            user_code
        );

        if copied_to_clipboard {
            println!("The code has been copied to your clipboard for convenience.")
        }

        // Open the default web browser to the device verification page, with
        // the user code copied to the clipboard.

        // TODO(radu): this works on macOS, but might fail on other systems (e.g. WSL2).
        // Also, there should be a way to disable it.

        // According to https://docs.rs/webbrowser/latest/webbrowser/ this should work on windows and Linux as well,
        // Tested on my linux VM and it worked
        let _ = webbrowser::open(&device_code.verification_url.clone().unwrap());

        // The OAuth library should theoretically handle waiting for the device to be authorized, but
        // testing revealed that it doesn't work. So we manually poll every 10 seconds for two minutes.
        let mut count = 0;
        let timeout = 12;

        // Loop while waiting for the device code to be authorized by the user
        loop {
            if count > timeout {
                bail!("Timed out waiting to authorize the device. Please execute the `fermyon login` command again and authorize the device with GitHub.");
            }

            match client.login(device_code.device_code.clone().unwrap()).await {
                // The cloud returns a 500 when the code is not authorized with a specific message, but when testing I only saw the response coming
                // back as Ok, but when the device code was not authorized the token was null
                // Expected behaviour would be that 500 lands in the Err
                Ok(response) => {
                    if response.token != None {
                        println!("Device authorized!");
                        return Ok(response);
                    }

                    println!("Waiting for device authorization...");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    count += 1;
                    continue;
                }
                Err(_) => {
                    println!("There was an error while waiting for device authorization");
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    count += 1;
                }
            };
        }
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

    async fn get_app_id(&self, hippo_client: &Client, name: String) -> Result<Uuid> {
        let apps_vm = Client::list_apps(hippo_client).await?;
        let app = apps_vm.items.iter().find(|&x| x.name == name.clone());
        match app {
            Some(a) => Ok(a.id),
            None => anyhow::bail!("No app with name: {}", name),
        }
    }

    async fn get_revision_id(
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
                anyhow::anyhow!(
                    "No revision with version {} and app id {}",
                    bindle_version,
                    app_id
                )
            })?
            .id)
    }

    async fn get_channel_id(
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
            None => anyhow::bail!("No channel with app_id {} and name {}", app_id, name),
        }
    }

    async fn create_and_push_bindle(&self, buildinfo: Option<BuildMetadata>) -> Result<Id> {
        let bindle_url = self.bindle_server_url.as_deref().unwrap();
        let source_dir = crate::app_dir(&self.app)?;
        let bindle_connection_info = spin_publish::BindleConnectionInfo::new(
            bindle_url,
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

        let _sloth_warning = warn_if_slow_response(bindle_url);

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
                        bindle_id, bindle_url
                    )
                });
            }
        }

        Ok(bindle_id.clone())
    }

    async fn check_hippo_healthz(&self) -> Result<()> {
        let hippo_base_url = url::Url::parse(&self.hippo_server_url)?;
        let hippo_healthz_url = hippo_base_url.join("/healthz")?;
        reqwest::get(hippo_healthz_url.to_string())
            .await?
            .error_for_status()
            .with_context(|| format!("Hippo server {} is unhealthy", hippo_base_url))?;
        Ok(())
    }
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

#[derive(Deserialize, Serialize)]
struct LoginHippoError {
    title: String,
    detail: String,
}

fn format_login_error(err: &anyhow::Error) -> anyhow::Result<String> {
    let error: LoginHippoError = serde_json::from_str(err.to_string().as_str())?;
    if error.detail.ends_with(": ") {
        Ok(format!(
            "Problem logging into Hippo: {}",
            error.detail.replace(": ", ".")
        ))
    } else {
        Ok(format!("Problem logging into Hippo: {}", error.detail))
    }
}

fn try_copy_to_clipboard(text: &str) -> bool {
    match ClipboardContext::new() {
        Ok(mut ctx) => {
            let result = ctx.set_contents(text.to_owned());
            result.is_ok()
        }
        Err(_) => false,
    }
}
