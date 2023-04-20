use anyhow::{Context, Result};
use cloud_openapi::{
    apis::{
        self,
        apps_api::{api_apps_get, api_apps_id_delete, api_apps_post},
        auth_tokens_api::api_auth_tokens_refresh_post,
        channels_api::{
            api_channels_get, api_channels_id_delete, api_channels_id_get,
            api_channels_id_logs_get, api_channels_post, ApiChannelsIdPatchError,
        },
        configuration::{ApiKey, Configuration},
        device_codes_api::api_device_codes_post,
        key_value_pairs_api::api_key_value_pairs_post,
        revisions_api::{api_revisions_get, api_revisions_post},
        Error, ResponseContent,
    },
    models::{
        AppItemPage, ChannelItem, ChannelItemPage, ChannelRevisionSelectionStrategy,
        CreateAppCommand, CreateChannelCommand, CreateDeviceCodeCommand, CreateKeyValuePairCommand,
        DeviceCodeItem, GetChannelLogsVm, RefreshTokenCommand, RegisterRevisionCommand,
        RevisionItemPage, TokenInfo, UpdateEnvironmentVariableDto,
    },
};
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

const JSON_MIME_TYPE: &str = "application/json";

pub struct Client {
    configuration: Configuration,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConnectionConfig {
    pub insecure: bool,
    pub token: String,
    pub url: String,
}

impl Client {
    pub fn new(conn_info: ConnectionConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, JSON_MIME_TYPE.parse().unwrap());
        headers.insert(header::CONTENT_TYPE, JSON_MIME_TYPE.parse().unwrap());

        let base_path = match conn_info.url.strip_suffix('/') {
            Some(s) => s.to_owned(),
            None => conn_info.url,
        };

        let configuration = Configuration {
            base_path,
            user_agent: Some(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            )),
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(conn_info.insecure)
                .default_headers(headers)
                .build()
                .unwrap(),
            basic_auth: None,
            oauth_access_token: None,
            bearer_access_token: None,
            api_key: Some(ApiKey {
                prefix: Some("Bearer".to_owned()),
                key: conn_info.token,
            }),
        };

        Self { configuration }
    }

    pub async fn create_device_code(&self, client_id: Uuid) -> Result<DeviceCodeItem> {
        api_device_codes_post(
            &self.configuration,
            CreateDeviceCodeCommand { client_id },
            None,
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn login(&self, token: String) -> Result<TokenInfo> {
        // When the new OpenAPI specification is released, manually crafting
        // the request should no longer be necessary.
        let response = self
            .configuration
            .client
            .post(format!("{}/api/auth-tokens", self.configuration.base_path))
            .body(
                serde_json::json!(
                    {
                        "provider": "DeviceFlow",
                        "clientId": "583e63e9-461f-4fbe-a246-23e0fb1cad10",
                        "providerCode": token,
                    }
                )
                .to_string(),
            )
            .send()
            .await?;

        serde_json::from_reader(response.bytes().await?.as_ref())
            .context("Failed to parse response")
    }

    pub async fn refresh_token(&self, token: String, refresh_token: String) -> Result<TokenInfo> {
        api_auth_tokens_refresh_post(
            &self.configuration,
            RefreshTokenCommand {
                token,
                refresh_token,
            },
            None,
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn add_app(&self, name: &str, storage_id: &str) -> Result<Uuid> {
        api_apps_post(
            &self.configuration,
            CreateAppCommand {
                name: name.to_string(),
                storage_id: storage_id.to_string(),
            },
            None,
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn remove_app(&self, id: String) -> Result<()> {
        api_apps_id_delete(&self.configuration, &id, None)
            .await
            .map_err(format_response_error)
    }

    pub async fn list_apps(&self) -> Result<AppItemPage> {
        api_apps_get(&self.configuration, None, None, None, None, None, None)
            .await
            .map_err(format_response_error)
    }

    pub async fn get_channel_by_id(&self, id: &str) -> Result<ChannelItem> {
        api_channels_id_get(&self.configuration, id, None)
            .await
            .map_err(format_response_error)
    }

    pub async fn list_channels(&self) -> Result<ChannelItemPage> {
        api_channels_get(
            &self.configuration,
            Some(""),
            None,
            None,
            Some("Name"),
            None,
            None,
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn list_channels_next(&self, previous: &ChannelItemPage) -> Result<ChannelItemPage> {
        api_channels_get(
            &self.configuration,
            Some(""),
            Some(previous.page_index + 1),
            Some(previous.page_size),
            Some("Name"),
            None,
            None,
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn add_channel(
        &self,
        app_id: Uuid,
        name: String,
        revision_selection_strategy: ChannelRevisionSelectionStrategy,
        range_rule: Option<String>,
        active_revision_id: Option<Uuid>,
    ) -> anyhow::Result<Uuid> {
        let command = CreateChannelCommand {
            app_id,
            name,
            revision_selection_strategy,
            range_rule: Some(range_rule),
            active_revision_id: Some(active_revision_id),
        };
        api_channels_post(&self.configuration, command, None)
            .await
            .map_err(format_response_error)
    }

    pub async fn patch_channel(
        &self,
        id: Uuid,
        name: Option<String>,
        revision_selection_strategy: Option<ChannelRevisionSelectionStrategy>,
        range_rule: Option<String>,
        active_revision_id: Option<Uuid>,
        environment_variables: Option<Vec<UpdateEnvironmentVariableDto>>,
    ) -> anyhow::Result<()> {
        let patch_channel_command = PatchChannelCommand {
            channel_id: Some(id),
            name,
            revision_selection_strategy,
            range_rule,
            active_revision_id,
            environment_variables,
        };

        let local_var_configuration = &self.configuration;

        let local_var_client = &local_var_configuration.client;

        let local_var_uri_str = format!(
            "{}/api/channels/{id}",
            local_var_configuration.base_path,
            id = apis::urlencode(id.to_string())
        );
        let mut local_var_req_builder =
            local_var_client.request(reqwest::Method::PATCH, local_var_uri_str.as_str());

        if let Some(ref local_var_user_agent) = local_var_configuration.user_agent {
            local_var_req_builder = local_var_req_builder
                .header(reqwest::header::USER_AGENT, local_var_user_agent.clone());
        }
        if let Some(ref local_var_apikey) = local_var_configuration.api_key {
            let local_var_key = local_var_apikey.key.clone();
            let local_var_value = match local_var_apikey.prefix {
                Some(ref local_var_prefix) => format!("{} {}", local_var_prefix, local_var_key),
                None => local_var_key,
            };
            local_var_req_builder = local_var_req_builder.header("Authorization", local_var_value);
        };
        local_var_req_builder = local_var_req_builder.json(&patch_channel_command);

        let local_var_req = local_var_req_builder.build()?;
        let local_var_resp = local_var_client.execute(local_var_req).await?;

        let local_var_status = local_var_resp.status();
        let local_var_content = local_var_resp.text().await?;

        if !local_var_status.is_client_error() && !local_var_status.is_server_error() {
            Ok(())
        } else {
            let local_var_entity: Option<ApiChannelsIdPatchError> =
                serde_json::from_str(&local_var_content).ok();
            let local_var_error = ResponseContent {
                status: local_var_status,
                content: local_var_content,
                entity: local_var_entity,
            };
            Err(format_response_error(Error::ResponseError(local_var_error)))
        }
    }

    pub async fn remove_channel(&self, id: String) -> Result<()> {
        api_channels_id_delete(&self.configuration, &id, None)
            .await
            .map_err(format_response_error)
    }

    pub async fn channel_logs(&self, id: String) -> Result<GetChannelLogsVm> {
        api_channels_id_logs_get(&self.configuration, &id, None, None)
            .await
            .map_err(format_response_error)
    }

    pub async fn add_revision(
        &self,
        app_storage_id: String,
        revision_number: String,
    ) -> anyhow::Result<()> {
        api_revisions_post(
            &self.configuration,
            RegisterRevisionCommand {
                app_storage_id,
                revision_number,
            },
            None,
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn list_revisions(&self) -> anyhow::Result<RevisionItemPage> {
        api_revisions_get(&self.configuration, None, None, None)
            .await
            .map_err(format_response_error)
    }

    pub async fn list_revisions_next(
        &self,
        previous: &RevisionItemPage,
    ) -> anyhow::Result<RevisionItemPage> {
        api_revisions_get(
            &self.configuration,
            Some(previous.page_index + 1),
            Some(previous.page_size),
            None,
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn add_key_value_pair(
        &self,
        app_id: Uuid,
        store_name: String,
        key: String,
        value: String,
    ) -> anyhow::Result<()> {
        api_key_value_pairs_post(
            &self.configuration,
            CreateKeyValuePairCommand {
                app_id,
                store_name,
                key,
                value,
            },
            None,
        )
        .await
        .map_err(format_response_error)
    }
}

#[derive(Deserialize, Debug)]
struct ValidationExceptionMessage {
    title: String,
    errors: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, Debug)]
struct CloudProblemDetails {
    detail: String,
}

fn format_response_error<T>(e: Error<T>) -> anyhow::Error {
    match e {
        Error::ResponseError(r) => {
            // Validation failures are distinguished by the presence of `errors` so try that first
            if let Ok(m) = serde_json::from_str::<ValidationExceptionMessage>(&r.content) {
                anyhow::anyhow!("{} {:?}", m.title, m.errors)
            } else if let Ok(d) = serde_json::from_str::<CloudProblemDetails>(&r.content) {
                anyhow::anyhow!("{}", d.detail)
            } else {
                anyhow::anyhow!("response status code: {}", r.status)
            }
        }
        Error::Serde(err) => {
            anyhow::anyhow!(format!("could not parse JSON object: {}", err))
        }
        _ => anyhow::anyhow!(e.to_string()),
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct PatchChannelCommand {
    #[serde(rename = "channelId", skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<uuid::Uuid>,
    #[serde(
        rename = "environmentVariables",
        skip_serializing_if = "Option::is_none"
    )]
    pub environment_variables: Option<Vec<UpdateEnvironmentVariableDto>>,
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        rename = "revisionSelectionStrategy",
        skip_serializing_if = "Option::is_none"
    )]
    pub revision_selection_strategy: Option<ChannelRevisionSelectionStrategy>,
    #[serde(rename = "rangeRule", skip_serializing_if = "Option::is_none")]
    pub range_rule: Option<String>,
    #[serde(rename = "activeRevisionId", skip_serializing_if = "Option::is_none")]
    pub active_revision_id: Option<uuid::Uuid>,
}

impl PatchChannelCommand {
    pub fn new() -> PatchChannelCommand {
        PatchChannelCommand {
            channel_id: None,
            environment_variables: None,
            name: None,
            revision_selection_strategy: None,
            range_rule: None,
            active_revision_id: None,
        }
    }
}
