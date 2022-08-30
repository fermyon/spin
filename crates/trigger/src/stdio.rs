use std::{collections::HashSet, fs::File, path::Path};

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
