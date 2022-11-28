use std::path::PathBuf;

use anyhow::Context;

pub(crate) struct TemplateOutputs {
    outputs: Vec<TemplateOutput>,
}

pub(crate) enum TemplateOutput {
    WriteFile(PathBuf, Vec<u8>),
    AppendToml(PathBuf, String),
}

impl TemplateOutputs {
    pub fn new(outputs: Vec<TemplateOutput>) -> Self {
        Self { outputs }
    }

    pub async fn write(&self) -> anyhow::Result<()> {
        for output in &self.outputs {
            output.write().await?;
        }
        Ok(())
    }
}

impl TemplateOutput {
    pub async fn write(&self) -> anyhow::Result<()> {
        match &self {
            TemplateOutput::WriteFile(path, contents) => {
                let dir = path.parent().with_context(|| {
                    format!("Can't get directory containing {}", path.display())
                })?;
                tokio::fs::create_dir_all(&dir)
                    .await
                    .with_context(|| format!("Failed to create directory {}", dir.display()))?;
                tokio::fs::write(&path, &contents)
                    .await
                    .with_context(|| format!("Failed to write file {}", path.display()))?;
            }
            TemplateOutput::AppendToml(path, text) => {
                let existing_toml = tokio::fs::read_to_string(path)
                    .await
                    .with_context(|| format!("Can't open {} to append", path.display()))?;
                let new_toml = format!("{}\n\n{}", existing_toml.trim_end(), text);
                tokio::fs::write(path, new_toml)
                    .await
                    .with_context(|| format!("Can't save changes to {}", path.display()))?;
            }
        }
        Ok(())
    }
}
