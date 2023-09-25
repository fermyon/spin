use serde::{Deserialize, Serialize};
use spin_app::MetadataKey;

/// Http trigger metadata key
pub const METADATA_KEY: MetadataKey<Metadata> = MetadataKey::new("trigger");

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    // The type of trigger which should always been "http" in this case
    pub r#type: String,
    // The based url
    #[serde(default = "default_base")]
    pub base: String,
}

pub fn default_base() -> String {
    "/".into()
}
