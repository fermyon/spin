use std::{fmt::Debug, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use dialoguer::{console::Emoji, Select};
use futures::FutureExt;
use spin_doctor::Diagnosis;

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
                        let desc = match treatment.description(patient).await {
                            Ok(desc) => desc,
                            Err(err) => {
                                show_error("Couldn't prepare treatment: ", err);
                                return Ok(());
                            }
                        };

                        let should_treat = prompt_treatment(desc).unwrap_or_else(|err| {
                            show_error("Prompt error: ", err);
                            false
                        });

                        if should_treat {
                            match treatment.treat(patient).await {
                                Ok(()) => {
                                    println!("{icon}Treatment applied!", icon = Emoji("❤️ ", ""));
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
            println!("{icon}No problems found.", icon = Emoji("❤️ ", ""));
        }
        Ok(())
    }
}

fn show_diagnosis(diagnosis: &dyn Diagnosis) {
    let icon = if diagnosis.is_critical() {
        Emoji("❗ ", "")
    } else {
        Emoji("⚠️ ", "")
    };
    println!("\n{icon}Diagnosis: {}", diagnosis.description());
}

fn prompt_treatment(desc: String) -> Result<bool> {
    let prompt = format!(
        "{icon}Treatment available! Would you like to apply it?",
        icon = Emoji("🩹 ", "")
    );
    let mut selection = Select::new()
        .with_prompt(prompt)
        .items(&["Yes", "No", "Describe treatment"])
        .default(0)
        .interact_opt()?;
    if selection == Some(2) {
        println!("\n{icon}{}\n", desc.trim_end(), icon = Emoji("📋 ", ""));
        selection = Select::new()
            .with_prompt("Would you like to apply this treatment?")
            .items(&["Yes", "No"])
            .default(0)
            .interact_opt()?
    }
    Ok(selection == Some(0))
}

fn show_error(prefix: &str, err: impl Debug) {
    let icon = Emoji("⁉️ ", "");
    println!("{icon}{prefix}{err:?}");
}
