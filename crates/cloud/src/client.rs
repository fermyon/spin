use anyhow::{Context, Result};
use cloud_openapi::{
    apis::{
        accounts_api::api_accounts_post,
        apps_api::{api_apps_get, api_apps_id_delete, api_apps_post},
        auth_tokens_api::api_auth_tokens_post,
        channels_api::{
            api_channels_get, api_channels_id_delete, api_channels_id_get,
            api_channels_id_logs_get, api_channels_id_patch, api_channels_post,
        },
        configuration::{ApiKey, Configuration},
        device_codes_api::api_device_codes_post,
        revisions_api::{api_revisions_get, api_revisions_post},
        Error,
    },
    models::{
        AppItemPage, ChannelItem, ChannelItemPage, ChannelRevisionSelectionStrategy,
        ChannelRevisionSelectionStrategyField, CreateAccountCommand, CreateAppCommand,
        CreateChannelCommand, CreateDeviceCodeCommand, CreateTokenCommand, DeviceCodeItem,
        GetChannelLogsVm, GuidNullableField, PatchChannelCommand, RegisterRevisionCommand,
        RevisionItemPage, StringField, TokenInfo, UpdateEnvironmentVariableDto,
        UpdateEnvironmentVariableDtoListField,
    },
};
use reqwest::header;
use semver::BuildMetadata;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tracing::log;
use uuid::Uuid;

const JSON_MIME_TYPE: &str = "application/json";

pub struct Client {
    configuration: Configuration,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConnectionConfig {
    pub insecure: bool,
    pub token: TokenInfo,
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
            api_key: conn_info.token.token.map(|t| ApiKey {
                prefix: Some("Bearer".to_owned()),
                key: t,
            }),
        };

        Self { configuration }
    }

    pub async fn create_device_code(&self, client_id: Uuid) -> Result<DeviceCodeItem> {
        api_device_codes_post(
            &self.configuration,
            Some(CreateDeviceCodeCommand { client_id }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn register(&self, username: String, password: String) -> Result<String> {
        api_accounts_post(
            &self.configuration,
            Some(CreateAccountCommand {
                user_name: username,
                password,
            }),
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

    pub async fn add_app(&self, name: &str, storage_id: &str) -> Result<Uuid> {
        api_apps_post(
            &self.configuration,
            Some(CreateAppCommand {
                name: name.to_string(),
                storage_id: storage_id.to_string(),
            }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn remove_app(&self, id: String) -> Result<()> {
        api_apps_id_delete(&self.configuration, &id)
            .await
            .map_err(format_response_error)
    }

    pub async fn list_apps(&self) -> Result<AppItemPage> {
        api_apps_get(&self.configuration, None, None, None, None, None)
            .await
            .map_err(format_response_error)
    }

    pub async fn get_channel_by_id(&self, id: &str) -> Result<ChannelItem> {
        api_channels_id_get(&self.configuration, id)
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
        )
        .await
        .map_err(format_response_error)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_channel(
        &self,
        app_id: Uuid,
        name: String,
        domain: Option<String>,
        revision_selection_strategy: ChannelRevisionSelectionStrategy,
        range_rule: Option<String>,
        active_revision_id: Option<Uuid>,
        certificate_id: Option<Uuid>,
    ) -> anyhow::Result<Uuid> {
        let command = CreateChannelCommand {
            app_id,
            name,
            domain,
            revision_selection_strategy,
            range_rule,
            active_revision_id,
            certificate_id,
        };
        api_channels_post(&self.configuration, Some(command))
            .await
            .map_err(format_response_error)
    }

    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub async fn patch_channel(
        &self,
        id: Uuid,
        name: Option<String>,
        domain: Option<String>,
        revision_selection_strategy: Option<ChannelRevisionSelectionStrategy>,
        range_rule: Option<String>,
        active_revision_id: Option<Uuid>,
        certificate_id: Option<Uuid>,
        environment_variables: Option<Vec<UpdateEnvironmentVariableDto>>,
    ) -> anyhow::Result<()> {
        let command = PatchChannelCommand {
            channel_id: Some(id),
            name: name.map(|n| Box::new(StringField { value: Some(n) })),
            domain: domain.map(|d| Box::new(StringField { value: Some(d) })),
            revision_selection_strategy: revision_selection_strategy
                .map(|r| Box::new(ChannelRevisionSelectionStrategyField { value: Some(r) })),
            range_rule: range_rule.map(|r| Box::new(StringField { value: Some(r) })),
            active_revision_id: active_revision_id
                .map(|r| Box::new(GuidNullableField { value: Some(r) })),
            certificate_id: certificate_id
                .map(|c| Box::new(GuidNullableField { value: Some(c) })),
            environment_variables: environment_variables
                .map(|e| Box::new(UpdateEnvironmentVariableDtoListField { value: Some(e) })),
        };

        api_channels_id_patch(&self.configuration, &id.to_string(), Some(command))
            .await
            .map_err(format_response_error)
    }

    pub async fn remove_channel(&self, id: String) -> Result<()> {
        api_channels_id_delete(&self.configuration, &id)
            .await
            .map_err(format_response_error)
    }

    pub async fn channel_logs(&self, id: String) -> Result<GetChannelLogsVm> {
        api_channels_id_logs_get(&self.configuration, &id)
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
            Some(RegisterRevisionCommand {
                app_storage_id,
                revision_number,
            }),
        )
        .await
        .map_err(format_response_error)
    }

    pub async fn list_revisions(&self) -> anyhow::Result<RevisionItemPage> {
        api_revisions_get(&self.configuration, None, None)
            .await
            .map_err(format_response_error)
    }
}

#[derive(Deserialize, Debug)]
struct ValidationExceptionMessage {
    title: String,
    errors: HashMap<String, Vec<String>>,
}

fn format_response_error<T>(e: Error<T>) -> anyhow::Error {
    match e {
        Error::ResponseError(r) => {
            match serde_json::from_str::<ValidationExceptionMessage>(&r.content) {
                Ok(m) => anyhow::anyhow!("{} {:?}", m.title, m.errors),
                _ => anyhow::anyhow!(r.content),
            }
        }
        Error::Serde(err) => {
            anyhow::anyhow!(format!("could not parse JSON object: {}", err))
        }
        _ => anyhow::anyhow!(e.to_string()),
    }
}
