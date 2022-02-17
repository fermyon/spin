use anyhow::Result;
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
    )]
    pub bindle_id: Option<bindle::Id>,

    /// URL of bindle server
    #[structopt(
        name = BINDLE_SERVER_URL_OPT,
        long = "bindle-server",
        env = BINDLE_URL_ENV,
    )]
    pub bindle_server_url: Option<String>,

    /// Temorary directory for the static assets of the components.
    #[structopt()]
    pub tmp: Option<PathBuf>,
}

impl Up {
    pub async fn run(self) -> Result<()> {
        let app = match (&self.app, &self.bindle_id) {
            (Some(app), None) => spin_loader::from_file(app, self.tmp).await?,
            _ => todo!("not implemented yet"),
        };

        let trigger = HttpTrigger::new(self.address, app, None).await?;
        trigger.run().await
    }
}
