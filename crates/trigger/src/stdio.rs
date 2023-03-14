use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{runtime_config::RuntimeConfig, TriggerHooks};

/// Which components should have their logs followed on stdout/stderr.
#[derive(Clone, Debug)]
pub enum FollowComponents {
    /// No components should have their logs followed.
    None,
    /// Only the specified components should have their logs followed.
    Named(HashSet<String>),
    /// All components should have their logs followed.
    All,
}

impl FollowComponents {
    /// Whether a given component should have its logs followed on stdout/stderr.
    pub fn should_follow(&self, component_id: &str) -> bool {
        match self {
            Self::None => false,
            Self::All => true,
            Self::Named(ids) => ids.contains(component_id),
        }
    }
}

impl Default for FollowComponents {
    fn default() -> Self {
        Self::None
    }
}

/// Implements TriggerHooks, writing logs to a log file and (optionally) stderr
pub struct StdioLoggingTriggerHooks {
    follow_components: FollowComponents,
    log_dir: Option<PathBuf>,
}

impl StdioLoggingTriggerHooks {
    pub fn new(follow_components: FollowComponents) -> Self {
        Self {
            follow_components,
            log_dir: None,
        }
    }

    fn component_stdio_writer(
        &self,
        component_id: &str,
        log_suffix: &str,
        log_dir: &Path,
    ) -> Result<ComponentStdioWriter> {
        let sanitized_component_id = sanitize_filename::sanitize(component_id);
        let log_path = log_dir.join(format!("{sanitized_component_id}_{log_suffix}.txt"));
        let follow = self.follow_components.should_follow(component_id);
        ComponentStdioWriter::new(&log_path, follow)
            .with_context(|| format!("Failed to open log file {log_path:?}"))
    }

    fn validate_follows(&self, app: &spin_app::App) -> anyhow::Result<()> {
        match &self.follow_components {
            FollowComponents::Named(names) => {
                let component_ids: HashSet<_> =
                    app.components().map(|c| c.id().to_owned()).collect();
                let unknown_names: Vec<_> = names.difference(&component_ids).collect();
                if unknown_names.is_empty() {
                    Ok(())
                } else {
                    let unknown_list = bullet_list(&unknown_names);
                    let actual_list = bullet_list(&component_ids);
                    let message = anyhow::anyhow!("The following component(s) specified in --follow do not exist in the application:\n{unknown_list}\nThe following components exist:\n{actual_list}");
                    Err(message)
                }
            }
            _ => Ok(()),
        }
    }
}

impl TriggerHooks for StdioLoggingTriggerHooks {
    fn app_loaded(
        &mut self,
        app: &spin_app::App,
        runtime_config: &RuntimeConfig,
    ) -> anyhow::Result<()> {
        self.log_dir = runtime_config.log_dir();

        self.validate_follows(app)?;

        if let Some(dir) = &self.log_dir {
            // Ensure log dir exists if set
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create log dir {dir:?}"))?;

            println!("Logging component stdio to {:?}", dir.join(""))
        }

        Ok(())
    }

    fn component_store_builder(
        &self,
        component: &spin_app::AppComponent,
        builder: &mut spin_core::StoreBuilder,
    ) -> anyhow::Result<()> {
        match &self.log_dir {
            Some(l) => {
                builder.stdout_pipe(self.component_stdio_writer(component.id(), "stdout", l)?);
                builder.stderr_pipe(self.component_stdio_writer(component.id(), "stderr", l)?);
            }
            None => {
                builder.inherit_stdout();
                builder.inherit_stderr();
            }
        }

        Ok(())
    }
}

/// ComponentStdioWriter forwards output to a log file and (optionally) stderr
pub struct ComponentStdioWriter {
    log_file: File,
    follow: bool,
}

impl ComponentStdioWriter {
    pub fn new(log_path: &Path, follow: bool) -> anyhow::Result<Self> {
        let log_file = File::options().create(true).append(true).open(log_path)?;
        Ok(Self { log_file, follow })
    }
}

impl std::io::Write for ComponentStdioWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.log_file.write(buf)?;
        if self.follow {
            std::io::stderr().write_all(&buf[..written])?;
        }
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.log_file.flush()?;
        if self.follow {
            std::io::stderr().flush()?;
        }
        Ok(())
    }
}

fn bullet_list<S: std::fmt::Display>(items: impl IntoIterator<Item = S>) -> String {
    items
        .into_iter()
        .map(|item| format!("  - {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}
