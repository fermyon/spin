use anyhow::Result;
use path_absolutize::Absolutize;
use spin_config::{ApplicationOrigin, Configuration, CoreComponent, RawConfiguration};
use spin_http_engine::HttpTrigger;
use std::{fs::File, io::Read, path::PathBuf};
use structopt::{clap::AppSettings, StructOpt};
use tracing::instrument;

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
    #[structopt(long = "app")]
    pub app: PathBuf,
}

impl Up {
    #[instrument]
    pub async fn run(self) -> Result<()> {
        let app = self.app_from_file()?;

        let trigger = HttpTrigger::new(self.address, app, None).await?;
        trigger.run().await
    }

    fn app_from_file(&self) -> Result<Configuration<CoreComponent>> {
        let mut buf = vec![];
        let mut file = File::open(&self.app)?;
        file.read_to_end(&mut buf)?;

        let absolute_app_path = self.app.absolutize()?.into_owned();

        let raw_app_config: RawConfiguration<CoreComponent> = toml::from_slice(&buf)?;
        let file_origin = ApplicationOrigin::File(absolute_app_path);
        Ok(Configuration::from_raw(raw_app_config, file_origin))
    }
}
