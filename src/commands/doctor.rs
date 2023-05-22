use std::{fmt::Debug, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use dialoguer::{console::Emoji, Confirm, Select};
use futures::FutureExt;
use spin_doctor::{Diagnosis, DryRunNotSupported};

use crate::opts::{APP_MANIFEST_FILE_OPT, DEFAULT_MANIFEST_FILE};

#[derive(Parser, Debug)]
#[clap(hide = true, about = "Detect and fix problems with Spin applications")]
pub struct DoctorCommand {
    /// The application to check. This may be a manifest (spin.toml) file, or a
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

    /// Run only the specified check.  You may specify this
    /// several times to run multiple checks.  The default is
    /// to run all checks.
    #[clap(long = "check", multiple = true)]
    checks: Vec<String>,
}

impl DoctorCommand {
    pub async fn run(self) -> Result<()> {
        let manifest_file = crate::manifest::resolve_file_path(&self.app_source)?;

        println!("{icon}The Spin Doctor is in.", icon = Emoji("📟 ", ""));
        println!(
            "{icon}Checking {}...",
            manifest_file.display(),
            icon = Emoji("🩺 ", "")
        );

        let count = spin_doctor::Checkup::new(manifest_file, &self.checks)
            .for_each_diagnosis(move |diagnosis, patient| {
                async move {
                    show_diagnosis(&*diagnosis);

                    if let Some(treatment) = diagnosis.treatment() {
                        let dry_run = match treatment.dry_run(patient).await {
                            Ok(desc) => Some(desc),
                            Err(err) => {
                                if !err.is::<DryRunNotSupported>() {
                                    show_error("Treatment dry run failed: ", err);
                                }
                                return Ok(());
                            }
                        };

                        let should_treat = prompt_treatment(treatment.summary(), dry_run)
                            .unwrap_or_else(|err| {
                                show_error("Prompt error: ", err);
                                false
                            });

                        if should_treat {
                            match treatment.treat(patient).await {
                                Ok(()) => {
                                    println!("{icon}Treatment applied!", icon = Emoji("❤  ", ""));
                                }
                                Err(err) => {
                                    show_error("Treatment failed: ", err);
                                }
                            }
                        }
                    }
                    Ok(())
                }
                .boxed()
            })
            .await?;
        if count == 0 {
            println!("{icon}No problems found.", icon = Emoji("❤  ", ""));
        }
        Ok(())
    }
}

fn show_diagnosis(diagnosis: &dyn Diagnosis) {
    let icon = if diagnosis.is_critical() {
        Emoji("❗ ", "")
    } else {
        Emoji("⚠  ", "")
    };
    println!("\n{icon}Diagnosis: {}", diagnosis.description());
}

fn prompt_treatment(summary: String, dry_run: Option<String>) -> Result<bool> {
    let prompt = format!(
        "{icon}The Spin Doctor can help! Would you like to",
        icon = Emoji("🩹 ", "")
    );
    let mut items = vec![summary.as_str(), "Do nothing"];
    if dry_run.is_some() {
        items.push("See more details about the recommended changes");
    }
    let selection = Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact_opt()?;

    match selection {
        Some(2) => {
            println!(
                "\n{icon}{}\n",
                dry_run.unwrap_or_default().trim_end(),
                icon = Emoji("📋 ", "")
            );
            Ok(Confirm::new()
                .with_prompt("Would you like to apply this fix?")
                .default(true)
                .interact_opt()?
                .unwrap_or_default())
        }
        Some(0) => Ok(true),
        _ => Ok(false),
    }
}

fn show_error(prefix: &str, err: impl Debug) {
    let icon = Emoji("⁉️ ", "");
    println!("{icon}{prefix}{err:?}");
}
