use spin_factors::anyhow;
use spin_world::{async_trait, v1, v2::variables};
use tracing::{instrument, Level};

use crate::InstanceState;

#[async_trait]
impl variables::Host for InstanceState {
    #[instrument(name = "spin_variables.get", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get(&mut self, key: String) -> Result<String, variables::Error> {
        let key = spin_expressions::Key::new(&key).map_err(expressions_to_variables_err)?;
        self.expression_resolver
            .resolve(&self.component_id, key)
            .await
            .map_err(expressions_to_variables_err)
    }

    fn convert_error(&mut self, error: variables::Error) -> anyhow::Result<variables::Error> {
        Ok(error)
    }
}

#[async_trait]
impl v1::config::Host for InstanceState {
    async fn get_config(&mut self, key: String) -> Result<String, v1::config::Error> {
        <Self as variables::Host>::get(self, key)
            .await
            .map_err(|err| match err {
                variables::Error::InvalidName(msg) => v1::config::Error::InvalidKey(msg),
                variables::Error::Undefined(msg) => v1::config::Error::Provider(msg),
                other => v1::config::Error::Other(format!("{other}")),
            })
    }

    fn convert_error(&mut self, err: v1::config::Error) -> anyhow::Result<v1::config::Error> {
        Ok(err)
    }
}

#[async_trait]
impl spin_world::wasi::config::store::Host for InstanceState {
    async fn get(
        &mut self,
        key: String,
    ) -> Result<Option<String>, spin_world::wasi::config::store::Error> {
        match <Self as variables::Host>::get(self, key).await {
            Ok(value) => Ok(Some(value)),
            // Err(e) if matches!(e, variables::Error::Undefined(_)) => Ok(Ok(None)),
            Err(e) => {
                match e {
                    variables::Error::Undefined(_) => Ok(None),
                    variables::Error::InvalidName(msg) => {
                        Err(spin_world::wasi::config::store::Error::Io(msg))
                    } // TODO: this doesn't feel ideal, but wasi-config only allows two error cases
                    variables::Error::Provider(msg) => {
                        Err(spin_world::wasi::config::store::Error::Upstream(msg))
                    }
                    variables::Error::Other(msg) => {
                        Err(spin_world::wasi::config::store::Error::Io(msg))
                    } // TODO: again, hard to know how to map this (and the original expressions::Error is not more helpful)
                }
            }
        }
    }

    async fn get_all(
        &mut self,
    ) -> Result<Vec<(String, String)>, spin_world::wasi::config::store::Error> {
        let all = self
            .expression_resolver
            .resolve_all(&self.component_id)
            .await;
        all.map_err(|e| {
            match expressions_to_variables_err(e) {
                variables::Error::Undefined(msg) => spin_world::wasi::config::store::Error::Io(msg), // TODO: this should presumably not occur here
                variables::Error::InvalidName(msg) => {
                    spin_world::wasi::config::store::Error::Io(msg)
                } // TODO: this doesn't feel ideal, but wasi-config only allows two error cases
                variables::Error::Provider(msg) => {
                    spin_world::wasi::config::store::Error::Upstream(msg)
                }
                variables::Error::Other(msg) => spin_world::wasi::config::store::Error::Io(msg), // TODO: again, hard to know how to map this (and the original expressions::Error is not more helpful)
            }
        })
    }

    fn convert_error(
        &mut self,
        err: spin_world::wasi::config::store::Error,
    ) -> anyhow::Result<spin_world::wasi::config::store::Error> {
        Ok(err)
    }
}

fn expressions_to_variables_err(err: spin_expressions::Error) -> variables::Error {
    use spin_expressions::Error;
    match err {
        Error::InvalidName(msg) => variables::Error::InvalidName(msg),
        Error::Undefined(msg) => variables::Error::Undefined(msg),
        Error::Provider(err) => variables::Error::Provider(err.to_string()),
        other => variables::Error::Other(format!("{other}")),
    }
}
