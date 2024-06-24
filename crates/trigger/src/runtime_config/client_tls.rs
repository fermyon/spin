#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub struct ClientTlsOpts {
    pub component_ids: Vec<String>,
    pub hosts: Vec<String>,
    pub custom_root_ca_file: Option<String>,
    pub cert_chain_file: Option<String>,
    pub private_key_file: Option<String>,
}
