use std::path::PathBuf;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

const DEFAULT_STORE_DIR: &str = "spin";
const DEFAULT_STORE_FILE: &str = "plugins-badger.json";

pub struct BadgerRecordManager {
    db_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct BadgerRecord {
    name: String,
    badgered_from: semver::Version,
    badgered_to: Vec<semver::Version>,
    when: chrono::DateTime<chrono::Utc>,
}

pub enum PreviousBadger {
    Fresh,
    FromCurrent {
        to: Vec<semver::Version>,
        when: chrono::DateTime<chrono::Utc>,
    },
}

impl PreviousBadger {
    fn includes(&self, version: &semver::Version) -> bool {
        match self {
            Self::Fresh => false,
            Self::FromCurrent { to, .. } => to.contains(version),
        }
    }

    pub fn includes_any(&self, version: &[&semver::Version]) -> bool {
        version.iter().any(|version| self.includes(version))
    }
}

impl BadgerRecordManager {
    pub fn default() -> anyhow::Result<Self> {
        let base_dir = dirs::cache_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".spin")))
            .ok_or_else(|| anyhow!("Unable to get local data directory or home directory"))?;
        let db_path = base_dir.join(DEFAULT_STORE_DIR).join(DEFAULT_STORE_FILE);
        Ok(Self { db_path })
    }

    fn load(&self) -> Vec<BadgerRecord> {
        match std::fs::read(&self.db_path) {
            Ok(v) => serde_json::from_slice(&v).unwrap_or_default(),
            Err(_) => vec![],
        }
    }

    fn save(&self, records: Vec<BadgerRecord>) -> anyhow::Result<()> {
        if let Some(dir) = self.db_path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(&records)?;
        std::fs::write(&self.db_path, json)?;
        Ok(())
    }

    async fn last_badger(&self, name: &str) -> Option<BadgerRecord> {
        self.load().into_iter().find(|r| r.name == name)
    }

    pub async fn previous_badger(
        &self,
        name: &str,
        current_version: &semver::Version,
    ) -> PreviousBadger {
        match self.last_badger(name).await {
            Some(b) if &b.badgered_from == current_version => PreviousBadger::FromCurrent {
                to: b.badgered_to,
                when: b.when,
            },
            _ => PreviousBadger::Fresh,
        }
    }

    pub async fn record_badger(&self, name: &str, from: &semver::Version, to: &[&semver::Version]) {
        let new = BadgerRecord {
            name: name.to_owned(),
            badgered_from: from.clone(),
            badgered_to: to.iter().map(|v| <semver::Version>::clone(v)).collect(),
            when: chrono::Utc::now(),
        };

        // There is a potential race condition here if someone runs two plugins at
        // the same time. As this is unlikely, and the worst outcome is that a user
        // misses a badger or gets a double badger, let's not worry about it for now.
        let mut existing = self.load();
        match existing.iter().position(|r| r.name == name) {
            Some(index) => existing[index] = new,
            None => existing.push(new),
        };
        _ = self.save(existing);
    }
}
