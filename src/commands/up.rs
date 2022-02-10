use anyhow::{Context, Result};
use spin_http_engine::HttpTrigger;
use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};

const APP_CONFIG_FILE_OPT: &str = "APP_CONFIG_FILE";
const BINDLE_ID_OPT: &str = "BINDLE_ID";
const BINDLE_SERVER_URL_OPT: &str = "BINDLE_SERVER_URL";

const BINDLE_URL_ENV: &str = "BINDLE_URL";

/// Start the Fermyon HTTP runtime.
#[derive(StructOpt, Debug)]
#[structopt(
    about = "Start the default HTTP listener",
    global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp]
)]
pub struct Up {
    /// IP address and port to listen on
    #[structopt(long = "listen", default_value = "127.0.0.1:3000")]
    pub address: String,

    /// Path to spin.toml
    /// TODO
    ///
    /// The command has to be run from the same directory
    /// as the configuration file for now.
    #[structopt(
        name = APP_CONFIG_FILE_OPT,
        long = "app",
        conflicts_with = BINDLE_ID_OPT,
    )]
    pub app: Option<PathBuf>,

    /// Id of application bindle
    #[structopt(
        name = BINDLE_ID_OPT,
        long = "bindle",
        conflicts_with = APP_CONFIG_FILE_OPT,
        requires = BINDLE_SERVER_URL_OPT,
        parse(try_from_str = try_bindle_id_from_str)
    )]
    pub bindle_id: Option<bindle::Id>,

    /// URL of bindle server
    #[structopt(
        name = BINDLE_SERVER_URL_OPT,
        long = "bindle-server",
        env = BINDLE_URL_ENV,
    )]
    pub bindle_server_url: Option<String>,
}

impl Up {
    pub async fn run(self) -> Result<()> {
        let app = match (&self.app, &self.bindle_id) {
            (None, None) =>
                Err(anyhow::anyhow!("Must specify app file or bindle id")),
            (Some(app_file), None) =>
                spin_config::read_from_file(app_file),
            (None, Some(bindle_id)) => {
                if let Some(server_url) = &self.bindle_server_url {
                    spin_config::read_from_bindle(bindle_id, server_url).await
                } else {
                    Err(anyhow::anyhow!("Loading from a bindle requires a Bindle server URL"))
                }
            },
            (Some(_), Some(_)) =>
                Err(anyhow::anyhow!("Specify only one of app file or bindle id")),
        }?;

        let trigger = HttpTrigger::new(self.address, app, None).await?;
        trigger.run().await
    }
}

fn try_bindle_id_from_str(id_str: &str) -> Result<bindle::Id> {
    let id = bindle::Id::try_from(id_str)
        .with_context(|| format!("'{}' is not a valid bindle ID", id_str))?;
    Ok(id)
}
