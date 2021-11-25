use crate::connection_from_env;
use anyhow::Result;
use hippo_client::{Client, ClientOptions};
use structopt::StructOpt;

/// Commands for working with Fermyon apps.
#[derive(StructOpt, Debug)]
pub enum AppCommands {
    /// Create a new application.
    New(New),
}

impl AppCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            AppCommands::New(cmd) => cmd.run().await,
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct New {
    /// The name of the application.
    pub name: String,

    /// Registry reference for the entrypoint component that
    /// implements the Fermyon interface.
    pub component: String,

    /// Default release channel of the application.
    #[structopt(default_value = "production")]
    pub channel: String,

    /// Default release channel of the application.
    #[structopt(default_value = "hippofactory.io")]
    pub domain: String,

    /// Optional revision to register for the application.
    #[structopt(long = "revision")]
    pub revision: Option<String>,
}

impl New {
    pub async fn run(self) -> Result<()> {
        let connection = connection_from_env()?;
        let options = ClientOptions {
            danger_accept_invalid_certs: true,
        };
        let client = Client::new_with_options(
            &connection.url,
            &connection.username,
            &connection.password,
            options,
        )
        .await?;

        let resp = client
            .create_application(&self.name, &self.component)
            .await?;

        println!(
            "Created new application {} from component {}: {}",
            self.name, self.component, resp.id
        );

        let subdomain = format!("{}.{}", &self.name, &self.domain);

        client
            .create_channel(&resp.id, &self.channel, &subdomain)
            .await?;

        println!(
            "Created new channel {} for application {}({}), on subdomain {}",
            &self.channel, &self.name, &resp.id, subdomain
        );

        if let Some(r) = self.revision {
            client
                .register_revision_by_storage_id(&self.component, &r)
                .await?;
        };

        Ok(())
    }
}
