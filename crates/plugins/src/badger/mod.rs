mod store;

use self::store::{BadgerRecordManager, PreviousBadger};
use crate::manifest::PluginManifest;
use is_terminal::IsTerminal;

const BADGER_TIMEOUT_DAYS: i64 = 14;

pub enum BadgerChecker {
    Precomputed(anyhow::Result<BadgerUI>),
    Deferred(BadgerEvaluator),
}

pub struct BadgerEvaluator {
    plugin_name: String,
    current_version: semver::Version,
    spin_version: &'static str,
    plugin_manager: crate::manager::PluginManager,
    record_manager: BadgerRecordManager,
    previous_badger: PreviousBadger,
}

impl BadgerChecker {
    pub fn start(
        name: &str,
        current_version: Option<String>,
        spin_version: &'static str,
    ) -> tokio::task::JoinHandle<Self> {
        let name = name.to_owned();

        tokio::task::spawn(async move {
            let current_version = match current_version {
                Some(v) => v.to_owned(),
                None => return Self::Precomputed(Ok(BadgerUI::None)),
            };

            if !std::io::stderr().is_terminal() {
                return Self::Precomputed(Ok(BadgerUI::None));
            }

            match BadgerEvaluator::new(&name, &current_version, spin_version).await {
                Ok(b) => {
                    if b.should_check() {
                        BadgerEvaluator::fire_and_forget_update();
                        Self::Deferred(b)
                    } else {
                        Self::Precomputed(Ok(BadgerUI::None))
                    }
                }
                Err(e) => Self::Precomputed(Err(e)),
            }
        })
    }

    pub async fn check(self) -> anyhow::Result<BadgerUI> {
        match self {
            Self::Precomputed(r) => r,
            Self::Deferred(b) => b.check().await,
        }
    }
}

impl BadgerEvaluator {
    async fn new(
        name: &str,
        current_version: &str,
        spin_version: &'static str,
    ) -> anyhow::Result<Self> {
        let current_version = semver::Version::parse(current_version)?;
        let plugin_manager = crate::manager::PluginManager::try_default()?;
        let record_manager = BadgerRecordManager::default()?;
        let previous_badger = record_manager.previous_badger(name, &current_version).await;

        Ok(Self {
            plugin_name: name.to_owned(),
            current_version,
            spin_version,
            plugin_manager,
            record_manager,
            previous_badger,
        })
    }

    fn should_check(&self) -> bool {
        match self.previous_badger {
            PreviousBadger::Fresh => true,
            PreviousBadger::FromCurrent { when, .. } => has_timeout_expired(when),
        }
    }

    fn fire_and_forget_update() {
        if let Err(e) = Self::fire_and_forget_update_impl() {
            tracing::info!("Failed to launch plugins update process; checking using latest local repo anyway. Error: {e:#}");
        }
    }

    fn fire_and_forget_update_impl() -> anyhow::Result<()> {
        let mut update_cmd = tokio::process::Command::new(std::env::current_exe()?);
        update_cmd.args(["plugins", "update"]);
        update_cmd.stdout(std::process::Stdio::null());
        update_cmd.stderr(std::process::Stdio::null());
        update_cmd.spawn()?;
        Ok(())
    }

    async fn check(&self) -> anyhow::Result<BadgerUI> {
        let available_upgrades = self.available_upgrades().await?;

        // TO CONSIDER: skipping this check and badgering for the same upgrade in case they missed it
        if self
            .previous_badger
            .includes_any(&available_upgrades.list())
        {
            return Ok(BadgerUI::None);
        }

        if !available_upgrades.is_none() {
            self.record_manager
                .record_badger(
                    &self.plugin_name,
                    &self.current_version,
                    &available_upgrades.list(),
                )
                .await
        };

        Ok(available_upgrades.classify())
    }

    async fn available_upgrades(&self) -> anyhow::Result<AvailableUpgrades> {
        let store = self.plugin_manager.store();

        let latest_version = {
            let latest_lookup = crate::lookup::PluginLookup::new(&self.plugin_name, None);
            let latest_manifest = latest_lookup
                .get_manifest_from_repository(store.get_plugins_directory())
                .await
                .ok();
            latest_manifest.and_then(|m| semver::Version::parse(m.version()).ok())
        };

        let manifests = store.catalogue_manifests()?;
        let relevant_manifests = manifests
            .into_iter()
            .filter(|m| m.name() == self.plugin_name);
        let compatible_manifests = relevant_manifests.filter(|m| {
            m.has_compatible_package() && m.is_compatible_spin_version(self.spin_version)
        });
        let compatible_plugin_versions =
            compatible_manifests.filter_map(|m| PluginVersion::try_from(m, &latest_version));
        let considerable_manifests = compatible_plugin_versions
            .filter(|pv| !pv.is_prerelease() && pv.is_higher_than(&self.current_version))
            .collect::<Vec<_>>();

        let (eligible_manifests, questionable_manifests) = if self.current_version.major == 0 {
            (vec![], considerable_manifests)
        } else {
            considerable_manifests
                .into_iter()
                .partition(|pv| pv.version.major == self.current_version.major)
        };

        let highest_eligible_manifest = eligible_manifests
            .into_iter()
            .max_by_key(|pv| pv.version.clone());
        let highest_questionable_manifest = questionable_manifests
            .into_iter()
            .max_by_key(|pv| pv.version.clone());

        Ok(AvailableUpgrades {
            eligible: highest_eligible_manifest,
            questionable: highest_questionable_manifest,
        })
    }
}

fn has_timeout_expired(from_time: chrono::DateTime<chrono::Utc>) -> bool {
    let timeout = chrono::Duration::days(BADGER_TIMEOUT_DAYS);
    let now = chrono::Utc::now();
    match now.checked_sub_signed(timeout) {
        None => true,
        Some(t) => from_time < t,
    }
}

pub struct AvailableUpgrades {
    eligible: Option<PluginVersion>,
    questionable: Option<PluginVersion>,
}

impl AvailableUpgrades {
    fn is_none(&self) -> bool {
        self.eligible.is_none() && self.questionable.is_none()
    }

    fn classify(&self) -> BadgerUI {
        match (&self.eligible, &self.questionable) {
            (None, None) => BadgerUI::None,
            (Some(e), None) => BadgerUI::Eligible(e.clone()),
            (None, Some(q)) => BadgerUI::Questionable(q.clone()),
            (Some(e), Some(q)) => BadgerUI::Both {
                eligible: e.clone(),
                questionable: q.clone(),
            },
        }
    }

    fn list(&self) -> Vec<&semver::Version> {
        [self.eligible.as_ref(), self.questionable.as_ref()]
            .iter()
            .filter_map(|pv| pv.as_ref())
            .map(|pv| &pv.version)
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct PluginVersion {
    version: semver::Version,
    name: String,
    is_latest: bool,
}

impl PluginVersion {
    fn try_from(manifest: PluginManifest, latest: &Option<semver::Version>) -> Option<Self> {
        match semver::Version::parse(manifest.version()) {
            Ok(version) => {
                let name = manifest.name();
                let is_latest = match latest {
                    None => false,
                    Some(latest) => &version == latest,
                };
                Some(Self {
                    version,
                    name,
                    is_latest,
                })
            }
            Err(_) => None,
        }
    }

    fn is_prerelease(&self) -> bool {
        !self.version.pre.is_empty()
    }

    fn is_higher_than(&self, other: &semver::Version) -> bool {
        &self.version > other
    }

    pub fn upgrade_command(&self) -> String {
        if self.is_latest {
            format!("spin plugins upgrade {}", self.name)
        } else {
            format!("spin plugins upgrade {} -v {}", self.name, self.version)
        }
    }
}

impl std::fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.version))
    }
}

pub enum BadgerUI {
    None,
    Eligible(PluginVersion),
    Questionable(PluginVersion),
    Both {
        eligible: PluginVersion,
        questionable: PluginVersion,
    },
}
