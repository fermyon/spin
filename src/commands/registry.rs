use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use spin_oci::Client;
use std::{io::Read, path::PathBuf};

use crate::opts::*;

/// Commands for working with OCI registries to distribute applications.
#[derive(Subcommand, Debug)]
pub enum RegistryCommands {
    /// Push a Spin application to a registry.
    Push(Push),
    /// Pull a Spin application from a registry.
    Pull(Pull),
    /// Log in to a registry.
    Login(Login),
}

impl RegistryCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            RegistryCommands::Push(cmd) => cmd.run().await,
            RegistryCommands::Pull(cmd) => cmd.run().await,
            RegistryCommands::Login(cmd) => cmd.run().await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct Push {
    /// Path to spin.toml
    #[clap(
        name = APP_MANIFEST_FILE_OPT,
        short = 'f',
        long = "file",
    )]
    pub app: Option<PathBuf>,

    /// Ignore server certificate errors
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Reference of the Spin application
    #[clap()]
    pub reference: String,
}

impl Push {
    pub async fn run(self) -> Result<()> {
        let app_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());

        let dir = tempfile::tempdir()?;
        let app = spin_loader::local::from_file(&app_file, Some(dir.path())).await?;

        let mut client = spin_oci::Client::new(self.insecure, None).await?;
        let digest = client.push(&app, &self.reference).await?;

        match digest {
            Some(digest) => println!("Pushed with digest {digest}"),
            None => println!("Pushed; the registry did not return the digest"),
        };

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Pull {
    /// Ignore server certificate errors
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Reference of the Spin application
    #[clap()]
    pub reference: String,
}

impl Pull {
    /// Pull a Spin application from an OCI registry
    pub async fn run(self) -> Result<()> {
        let mut client = spin_oci::Client::new(self.insecure, None).await?;
        client.pull(&self.reference).await?;

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Login {
    /// Username for the registry
    #[clap(long = "username", short = 'u')]
    pub username: Option<String>,

    /// Password for the registry
    #[clap(long = "password", short = 'p')]
    pub password: Option<String>,

    /// Take the password from stdin
    #[clap(
        long = "password-stdin",
        takes_value = false,
        conflicts_with = "password"
    )]
    pub password_stdin: bool,

    #[clap()]
    pub server: String,
}

impl Login {
    pub async fn run(self) -> Result<()> {
        let username = match self.username {
            Some(u) => u,
            None => {
                let prompt = "Username";
                loop {
                    let result = dialoguer::Input::<String>::new()
                        .with_prompt(prompt)
                        .interact_text()?;
                    if result.trim().is_empty() {
                        continue;
                    } else {
                        break result;
                    }
                }
            }
        };

        // If the --password-stdin flag is passed, read the password from standard input.
        // Otherwise, if the --password flag was passed with a value, use that value. Finally, if
        // neither was passed, prompt the user to input the password.
        let password = if self.password_stdin {
            let mut buf = String::new();
            let mut stdin = std::io::stdin().lock();
            stdin.read_to_string(&mut buf)?;
            buf
        } else {
            match self.password {
                Some(p) => p,
                None => rpassword::prompt_password("Password: ")?,
            }
        };

        Client::login(&self.server, &username, &password)
            .await
            .context("cannot log in to the registry")?;

        println!(
            "Successfully logged in as {} to registry {}",
            username, &self.server
        );
        Ok(())
    }
}
