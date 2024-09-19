use std::path::PathBuf;

use anyhow::Context;

pub(crate) struct TemplateOutputs {
    outputs: Vec<TemplateOutput>,
}

pub(crate) enum TemplateOutput {
    WriteFile(PathBuf, Vec<u8>),
    AppendToml(PathBuf, String),
    MergeToml(PathBuf, &'static str, String), // only have to worry about merging into root table for now
    CreateDirectory(PathBuf),
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
            TemplateOutput::MergeToml(path, target, text) => {
                let existing_toml = tokio::fs::read_to_string(path)
                    .await
                    .with_context(|| format!("Can't open {} to append", path.display()))?;
                let new_toml = merge_toml(&existing_toml, target, text)?;
                tokio::fs::write(path, new_toml)
                    .await
                    .with_context(|| format!("Can't save changes to {}", path.display()))?;
            }
            TemplateOutput::CreateDirectory(dir) => {
                tokio::fs::create_dir_all(dir)
                    .await
                    .with_context(|| format!("Failed to create directory {}", dir.display()))?;
            }
        }
        Ok(())
    }
}

fn merge_toml(existing: &str, target: &str, text: &str) -> anyhow::Result<String> {
    use toml_edit::{DocumentMut, Entry, Item};

    let mut doc: DocumentMut = existing
        .parse()
        .context("Can't merge into the existing manifest - it's not valid TOML")?;
    let merging: DocumentMut = text
        .parse()
        .context("Can't merge snippet - it's not valid TOML")?;
    let merging = merging.as_table();
    match doc.get_mut(target) {
        Some(item) => {
            let Some(table) = item.as_table_mut() else {
                anyhow::bail!("Cannot merge template data into {target} as it is not a table");
            };
            for (key, value) in merging {
                match table.entry(key) {
                    Entry::Occupied(mut e) => {
                        let existing_val = e.get_mut();
                        *existing_val = value.clone();
                    }
                    Entry::Vacant(e) => {
                        e.insert(value.clone());
                    }
                }
            }
        }
        None => {
            let table = Item::Table(merging.clone());
            doc.insert(target, table);
        }
    };
    Ok(doc.to_string())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_insert_variables_in_manifest() {
        let manifest = r#"spin_version = "1"

[[component]]
id = "dummy"
"#;

        let variables = r#"url = { required = true }"#;

        let new = merge_toml(manifest, "variables", variables).unwrap();

        assert_eq!(
            r#"spin_version = "1"

[[component]]
id = "dummy"

[variables]
url = { required = true }
"#,
            new
        );
    }

    #[test]
    fn can_merge_variables_into_manifest() {
        let manifest = r#"spin_version = "1"

[variables]
secret = { default = "1234 but don't tell anyone!" }

[[component]]
id = "dummy"
"#;

        let variables = r#"url = { required = true }"#;

        let new = merge_toml(manifest, "variables", variables).unwrap();

        assert_eq!(
            r#"spin_version = "1"

[variables]
secret = { default = "1234 but don't tell anyone!" }
url = { required = true }

[[component]]
id = "dummy"
"#,
            new
        );
    }
}
