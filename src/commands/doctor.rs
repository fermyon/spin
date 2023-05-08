use std::{fmt::Debug, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use dialoguer::{console::Emoji, Confirm, Select};
use futures::FutureExt;
use spin_doctor::{Diagnosis, DryRunNotSupported};

#[derive(Parser, Debug)]
#[clap(hide = true, about = "Detect and fix problems with Spin applications")]
pub struct DoctorCommand {
    #[clap(short = 'f', long, default_value = "spin.toml")]
    file: PathBuf,
}

impl DoctorCommand {
    pub async fn run(self) -> Result<()> {
        println!("{icon}The Spin Doctor is in.", icon = Emoji("📟 ", ""));
        println!(
            "{icon}Checking {}...",
            self.file.display(),
            icon = Emoji("🩺 ", "")
        );

        let count = spin_doctor::Checkup::new(self.file)
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
