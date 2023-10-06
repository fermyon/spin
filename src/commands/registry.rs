use crate::opts::*;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use spin_oci::Client;
use std::{io::Read, path::PathBuf, time::Duration};

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
    /// The application to push. This may be a manifest (spin.toml) file, or a
    /// directory containing a spin.toml file.
    /// If omitted, it defaults to "spin.toml".
    #[clap(
        name = APP_MANIFEST_FILE_OPT,
        short = 'f',
        long = "from",
        alias = "file",
        value_hint = clap::ValueHint::FilePath,
        default_value = DEFAULT_MANIFEST_FILE
    )]
    pub app_source: PathBuf,

    /// Ignore server certificate errors
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Specifies to perform `spin build` before pushing the application.
    #[clap(long, takes_value = false, env = ALWAYS_BUILD_ENV)]
    pub build: bool,

    /// Reference of the Spin application
    #[clap()]
    pub reference: String,
}

impl Push {
    pub async fn run(self) -> Result<()> {
        let app_file = spin_common::paths::resolve_manifest_file_path(&self.app_source)?;
        if self.build {
            spin_build::build(&app_file, &[]).await?;
        }

        let dir = tempfile::tempdir()?;
        let app = spin_loader::local::from_file(&app_file, Some(dir.path())).await?;

        let mut client = spin_oci::Client::new(self.insecure, None).await?;

        let _spinner = create_dotted_spinner(2000, "Pushing app to the Registry".to_owned());

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

        let _spinner = create_dotted_spinner(2000, "Pulling app from the Registry".to_owned());

        client.pull(&self.reference).await?;
        println!("Successfully pulled the app from the registry");
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

fn create_dotted_spinner(interval: u64, message: String) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(Duration::from_millis(interval));
    spinner.set_style(
        ProgressStyle::with_template("{msg}{spinner}\n")
            .unwrap()
            .tick_strings(&[".", "..", "...", "....", "....."]),
    );
    spinner.set_message(message);
    spinner
}
