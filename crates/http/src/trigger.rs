use serde::{Deserialize, Serialize};
use spin_locked_app::MetadataKey;

/// Http trigger metadata key
pub const METADATA_KEY: MetadataKey<Metadata> = MetadataKey::new("trigger");

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    // The based url
    #[serde(default = "default_base")]
    pub base: String,
}

pub fn default_base() -> String {
    "/".into()
}
