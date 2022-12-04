use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{TriggerHooks, SPIN_HOME};

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
    pub fn new(follow_components: FollowComponents, log_dir: Option<PathBuf>) -> Self {
        Self {
            follow_components,
            log_dir,
        }
    }

    fn component_stdio_writer(
        &self,
        component_id: &str,
        log_suffix: &str,
    ) -> Result<ComponentStdioWriter> {
        let sanitized_component_id = sanitize_filename::sanitize(component_id);
        let log_path = self
            .log_dir
            .as_deref()
            .expect("log_dir should have been initialized in app_loaded")
            .join(format!("{sanitized_component_id}_{log_suffix}.txt"));
        let follow = self.follow_components.should_follow(component_id);
        ComponentStdioWriter::new(&log_path, follow)
            .with_context(|| format!("Failed to open log file {log_path:?}"))
    }

    fn create_log_dir(&mut self, app: &spin_app::App) -> anyhow::Result<()> {
        let app_name: &str = app.require_metadata("name")?;

        // Set default log_dir (if not explicitly passed)
        let log_dir = self
            .log_dir
            .get_or_insert_with(|| default_log_dir(app_name));

        // Ensure log dir exists
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log dir {log_dir:?}"))?;

        Ok(())
    }
}

impl TriggerHooks for StdioLoggingTriggerHooks {
    fn app_loaded(&mut self, app: &spin_app::App) -> anyhow::Result<()> {
        self.create_log_dir(app)?;

        Ok(())
    }

    fn component_store_builder(
        &self,
        component: spin_app::AppComponent,
        builder: &mut spin_core::StoreBuilder,
    ) -> anyhow::Result<()> {
        builder.stdout_pipe(self.component_stdio_writer(component.id(), "stdout")?);
        builder.stderr_pipe(self.component_stdio_writer(component.id(), "stderr")?);
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

fn default_log_dir(app_name: &str) -> PathBuf {
    let parent_dir = match dirs::home_dir() {
        Some(home) => home.join(SPIN_HOME),
        None => PathBuf::new(), // "./"
    };
    let sanitized_app = sanitize_filename::sanitize(app_name);

    parent_dir.join(sanitized_app).join("logs")
}
