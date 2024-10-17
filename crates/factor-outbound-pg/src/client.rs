use anyhow::{anyhow, Result};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use spin_world::async_trait;
use spin_world::spin::postgres::rdbms_types::{
    self as v2, Column, DbDataType, DbValue, ParameterValue, RowSet,
};
use tokio_postgres::types::Type;
use tokio_postgres::{config::SslMode, types::ToSql, Row};
use tokio_postgres::{Client as TokioClient, NoTls, Socket};

#[async_trait]
pub trait Client {
    async fn build_client(address: &str) -> Result<Self>
    where
        Self: Sized;

    async fn execute(
        &self,
        statement: String,
        params: Vec<ParameterValue>,
    ) -> Result<u64, v2::Error>;

    async fn query(
        &self,
        statement: String,
        params: Vec<ParameterValue>,
    ) -> Result<RowSet, v2::Error>;
}

#[async_trait]
impl Client for TokioClient {
    async fn build_client(address: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let config = address.parse::<tokio_postgres::Config>()?;

        tracing::debug!("Build new connection: {}", address);

        if config.get_ssl_mode() == SslMode::Disable {
            let (client, connection) = config.connect(NoTls).await?;
            spawn_connection(connection);
            Ok(client)
        } else {
            let builder = TlsConnector::builder();
            let connector = MakeTlsConnector::new(builder.build()?);
            let (client, connection) = config.connect(connector).await?;
            spawn_connection(connection);
            Ok(client)
        }
    }

    async fn execute(
        &self,
        statement: String,
        params: Vec<ParameterValue>,
    ) -> Result<u64, v2::Error> {
        let params = params
            .iter()
            .map(to_sql_parameter)
            .collect::<Result<Vec<_>>>()
            .map_err(|e| v2::Error::ValueConversionFailed(format!("{:?}", e)))?;

        let params_refs: Vec<&(dyn ToSql + Sync)> = params
            .iter()
            .map(|b| b.as_ref() as &(dyn ToSql + Sync))
            .collect();

        self.execute(&statement, params_refs.as_slice())
            .await
            .map_err(|e| v2::Error::QueryFailed(format!("{:?}", e)))
    }

    async fn query(
        &self,
        statement: String,
        params: Vec<ParameterValue>,
    ) -> Result<RowSet, v2::Error> {
        let params = params
            .iter()
            .map(to_sql_parameter)
            .collect::<Result<Vec<_>>>()
            .map_err(|e| v2::Error::BadParameter(format!("{:?}", e)))?;

        let params_refs: Vec<&(dyn ToSql + Sync)> = params
            .iter()
            .map(|b| b.as_ref() as &(dyn ToSql + Sync))
            .collect();

        let results = self
            .query(&statement, params_refs.as_slice())
            .await
            .map_err(|e| v2::Error::QueryFailed(format!("{:?}", e)))?;

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
            .map_err(|e| v2::Error::QueryFailed(format!("{:?}", e)))?;

        Ok(RowSet { columns, rows })
    }
}

fn spawn_connection<T>(connection: tokio_postgres::Connection<Socket, T>)
where
    T: tokio_postgres::tls::TlsStream + std::marker::Unpin + std::marker::Send + 'static,
{
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Postgres connection error: {}", e);
        }
    });
}

fn to_sql_parameter(value: &ParameterValue) -> Result<Box<dyn ToSql + Send + Sync>> {
    match value {
        ParameterValue::Boolean(v) => Ok(Box::new(*v)),
        ParameterValue::Int32(v) => Ok(Box::new(*v)),
        ParameterValue::Int64(v) => Ok(Box::new(*v)),
        ParameterValue::Int8(v) => Ok(Box::new(*v)),
        ParameterValue::Int16(v) => Ok(Box::new(*v)),
        ParameterValue::Floating32(v) => Ok(Box::new(*v)),
        ParameterValue::Floating64(v) => Ok(Box::new(*v)),
        ParameterValue::Uint8(_)
        | ParameterValue::Uint16(_)
        | ParameterValue::Uint32(_)
        | ParameterValue::Uint64(_) => Err(anyhow!("Postgres does not support unsigned integers")),
        ParameterValue::Str(v) => Ok(Box::new(v.clone())),
        ParameterValue::Binary(v) => Ok(Box::new(v.clone())),
        ParameterValue::Date((y, mon, d)) => {
            let naive_date = chrono::NaiveDate::from_ymd_opt(*y, (*mon).into(), (*d).into())
                .ok_or_else(|| anyhow!("invalid date y={y}, m={mon}, d={d}"))?;
            Ok(Box::new(naive_date))
        }
        ParameterValue::Time((h, min, s, ns)) => {
            let naive_time =
                chrono::NaiveTime::from_hms_nano_opt((*h).into(), (*min).into(), (*s).into(), *ns)
                    .ok_or_else(|| anyhow!("invalid time {h}:{min}:{s}:{ns}"))?;
            Ok(Box::new(naive_time))
        }
        ParameterValue::Datetime((y, mon, d, h, min, s, ns)) => {
            let naive_date = chrono::NaiveDate::from_ymd_opt(*y, (*mon).into(), (*d).into())
                .ok_or_else(|| anyhow!("invalid date y={y}, m={mon}, d={d}"))?;
            let naive_time =
                chrono::NaiveTime::from_hms_nano_opt((*h).into(), (*min).into(), (*s).into(), *ns)
                    .ok_or_else(|| anyhow!("invalid time {h}:{min}:{s}:{ns}"))?;
            let dt = chrono::NaiveDateTime::new(naive_date, naive_time);
            Ok(Box::new(dt))
        }
        ParameterValue::Timestamp(v) => {
            let ts = chrono::DateTime::<chrono::Utc>::from_timestamp(*v, 0)
                .ok_or_else(|| anyhow!("invalid epoch timestamp {v}"))?;
            Ok(Box::new(ts))
        }
        ParameterValue::DbNull => Ok(Box::new(PgNull)),
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

/// Although the Postgres crate converts Rust Option::None to Postgres NULL,
/// it enforces the type of the Option as it does so. (For example, trying to
/// pass an Option::<i32>::None to a VARCHAR column fails conversion.) As we
/// do not know expected column types, we instead use a "neutral" custom type
/// which allows conversion to any type but always tells the Postgres crate to
/// treat it as a SQL NULL.
struct PgNull;

impl ToSql for PgNull {
    fn to_sql(
        &self,
        _ty: &Type,
        _out: &mut tokio_postgres::types::private::BytesMut,
    ) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        Ok(tokio_postgres::types::IsNull::Yes)
    }

    fn accepts(_ty: &Type) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn to_sql_checked(
        &self,
        _ty: &Type,
        _out: &mut tokio_postgres::types::private::BytesMut,
    ) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        Ok(tokio_postgres::types::IsNull::Yes)
    }
}

impl std::fmt::Debug for PgNull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NULL").finish()
    }
}
