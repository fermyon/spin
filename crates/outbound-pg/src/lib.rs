use anyhow::{anyhow, Result};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use spin_core::{
    async_trait,
    postgres::{self, PgError},
    rdbms_types::{Column, DbDataType, DbValue, ParameterValue, RowSet},
    HostComponent,
};
use std::collections::HashMap;
use tokio_postgres::{
    config::SslMode,
    types::{ToSql, Type},
    Client, NoTls, Row, Socket,
};

/// A simple implementation to support outbound pg connection
#[derive(Default)]
pub struct OutboundPg {
    pub connections: HashMap<String, Client>,
}

impl HostComponent for OutboundPg {
    type Data = Self;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        postgres::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

#[async_trait]
impl postgres::Host for OutboundPg {
    async fn execute(
        &mut self,
        address: String,
        statement: String,
        params: Vec<ParameterValue>,
    ) -> Result<Result<u64, PgError>> {
        Ok(async {
            let params: Vec<&(dyn ToSql + Sync)> = params
                .iter()
                .map(to_sql_parameter)
                .collect::<anyhow::Result<Vec<_>>>()
                .map_err(|e| PgError::ValueConversionFailed(format!("{:?}", e)))?;

            let nrow = self
                .get_client(&address)
                .await
                .map_err(|e| PgError::ConnectionFailed(format!("{:?}", e)))?
                .execute(&statement, params.as_slice())
                .await
                .map_err(|e| PgError::QueryFailed(format!("{:?}", e)))?;

            Ok(nrow)
        }
        .await)
    }

    async fn query(
        &mut self,
        address: String,
        statement: String,
        params: Vec<ParameterValue>,
    ) -> Result<Result<RowSet, PgError>> {
        Ok(async {
            let params: Vec<&(dyn ToSql + Sync)> = params
                .iter()
                .map(to_sql_parameter)
                .collect::<anyhow::Result<Vec<_>>>()
                .map_err(|e| PgError::BadParameter(format!("{:?}", e)))?;

            let results = self
                .get_client(&address)
                .await
                .map_err(|e| PgError::ConnectionFailed(format!("{:?}", e)))?
                .query(&statement, params.as_slice())
                .await
                .map_err(|e| PgError::QueryFailed(format!("{:?}", e)))?;

            if results.is_empty() {
                return Ok(RowSet {
                    columns: vec![],
                    rows: vec![],
                });
            }

            let columns = infer_columns(&results[0]);
            let rows = results
                .iter()
                .map(convert_row)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| PgError::QueryFailed(format!("{:?}", e)))?;

            Ok(RowSet { columns, rows })
        }
        .await)
    }
}

const DB_NULL: Option<i32> = None;

fn to_sql_parameter(value: &ParameterValue) -> anyhow::Result<&(dyn ToSql + Sync)> {
    match value {
        ParameterValue::Boolean(v) => Ok(v),
        ParameterValue::Int32(v) => Ok(v),
        ParameterValue::Int64(v) => Ok(v),
        ParameterValue::Int8(v) => Ok(v),
        ParameterValue::Int16(v) => Ok(v),
        ParameterValue::Floating32(v) => Ok(v),
        ParameterValue::Floating64(v) => Ok(v),
        ParameterValue::Uint8(_)
        | ParameterValue::Uint16(_)
        | ParameterValue::Uint32(_)
        | ParameterValue::Uint64(_) => Err(anyhow!("Postgres does not support unsigned integers")),
        ParameterValue::Str(v) => Ok(v),
        ParameterValue::Binary(v) => Ok(v),
        ParameterValue::DbNull => Ok(&DB_NULL),
    }
}

fn infer_columns(row: &Row) -> Vec<Column> {
    let mut result = Vec::with_capacity(row.len());
    for index in 0..row.len() {
        result.push(infer_column(row, index));
    }
    result
}

fn infer_column(row: &Row, index: usize) -> Column {
    let column = &row.columns()[index];
    let name = column.name().to_owned();
    let data_type = convert_data_type(column.type_());
    Column { name, data_type }
}

fn convert_data_type(pg_type: &Type) -> DbDataType {
    match *pg_type {
        Type::BOOL => DbDataType::Boolean,
        Type::BYTEA => DbDataType::Binary,
        Type::FLOAT4 => DbDataType::Floating32,
        Type::FLOAT8 => DbDataType::Floating64,
        Type::INT2 => DbDataType::Int16,
        Type::INT4 => DbDataType::Int32,
        Type::INT8 => DbDataType::Int64,
        Type::TEXT | Type::VARCHAR | Type::BPCHAR => DbDataType::Str,
        _ => {
            tracing::debug!("Couldn't convert Postgres type {} to WIT", pg_type.name(),);
            DbDataType::Other
        }
    }
}

fn convert_row(row: &Row) -> Result<Vec<DbValue>, tokio_postgres::Error> {
    let mut result = Vec::with_capacity(row.len());
    for index in 0..row.len() {
        result.push(convert_entry(row, index)?);
    }
    Ok(result)
}

fn convert_entry(row: &Row, index: usize) -> Result<DbValue, tokio_postgres::Error> {
    let column = &row.columns()[index];
    let value = match column.type_() {
        &Type::BOOL => {
            let value: Option<bool> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Boolean(v),
                None => DbValue::DbNull,
            }
        }
        &Type::BYTEA => {
            let value: Option<Vec<u8>> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Binary(v),
                None => DbValue::DbNull,
            }
        }
        &Type::FLOAT4 => {
            let value: Option<f32> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Floating32(v),
                None => DbValue::DbNull,
            }
        }
        &Type::FLOAT8 => {
            let value: Option<f64> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Floating64(v),
                None => DbValue::DbNull,
            }
        }
        &Type::INT2 => {
            let value: Option<i16> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Int16(v),
                None => DbValue::DbNull,
            }
        }
        &Type::INT4 => {
            let value: Option<i32> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Int32(v),
                None => DbValue::DbNull,
            }
        }
        &Type::INT8 => {
            let value: Option<i64> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Int64(v),
                None => DbValue::DbNull,
            }
        }
        &Type::TEXT | &Type::VARCHAR | &Type::BPCHAR => {
            let value: Option<String> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Str(v),
                None => DbValue::DbNull,
            }
        }
        t => {
            tracing::debug!(
                "Couldn't convert Postgres type {} in column {}",
                t.name(),
                column.name()
            );
            DbValue::Unsupported
        }
    };
    Ok(value)
}

impl OutboundPg {
    async fn get_client(&mut self, address: &str) -> anyhow::Result<&Client> {
        let client = match self.connections.entry(address.to_owned()) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => v.insert(build_client(address).await?),
        };
        Ok(client)
    }
}

async fn build_client(address: &str) -> anyhow::Result<Client> {
    let config = address.parse::<tokio_postgres::Config>()?;

    tracing::debug!("Build new connection: {}", address);

    if config.get_ssl_mode() == SslMode::Disable {
        connect(config).await
    } else {
        connect_tls(config).await
    }
}

async fn connect(config: tokio_postgres::Config) -> anyhow::Result<Client> {
    let (client, connection) = config.connect(NoTls).await?;

    spawn(connection);

    Ok(client)
}

async fn connect_tls(config: tokio_postgres::Config) -> anyhow::Result<Client> {
    let builder = TlsConnector::builder();
    let connector = MakeTlsConnector::new(builder.build()?);
    let (client, connection) = config.connect(connector).await?;

    spawn(connection);

    Ok(client)
}

fn spawn<T>(connection: tokio_postgres::Connection<Socket, T>)
where
    T: tokio_postgres::tls::TlsStream + std::marker::Unpin + std::marker::Send + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Postgres connection error: {}", e);
        }
    });
}
