use anyhow::anyhow;
use outbound_pg::*;
use tokio_postgres::{
    tls::NoTlsStream,
    types::{ToSql, Type},
    Connection, NoTls, Row, Socket,
};

pub use outbound_pg::add_to_linker;
use spin_engine::{
    host_component::{HostComponent, HostComponentsStateHandle},
    RuntimeContext,
};
use wit_bindgen_wasmtime::{async_trait, wasmtime::Linker};

wit_bindgen_wasmtime::export!({paths: ["../../wit/ephemeral/outbound-pg.wit"], async: *});

/// A simple implementation to support outbound pg connection
#[derive(Default, Clone)]
pub struct OutboundPg;

impl HostComponent for OutboundPg {
    type State = Self;

    fn add_to_linker<T: Send>(
        linker: &mut Linker<RuntimeContext<T>>,
        state_handle: HostComponentsStateHandle<Self::State>,
    ) -> anyhow::Result<()> {
        add_to_linker(linker, move |ctx| state_handle.get_mut(ctx))
    }

    fn build_state(
        &self,
        _component: &spin_manifest::CoreComponent,
    ) -> anyhow::Result<Self::State> {
        Ok(Self)
    }
}

#[async_trait]
impl outbound_pg::OutboundPg for OutboundPg {
    async fn execute(
        &mut self,
        address: &str,
        statement: &str,
        params: Vec<ParameterValue<'_>>,
    ) -> Result<u64, PgError> {
        let (client, connection) = tokio_postgres::connect(address, NoTls)
            .await
            .map_err(|e| PgError::ConnectionFailed(format!("{:?}", e)))?;

        spawn(connection);

        let params: Vec<&(dyn ToSql + Sync)> = params
            .iter()
            .map(to_sql_parameter)
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(|e| PgError::ValueConversionFailed(format!("{:?}", e)))?;

        let nrow = client
            .execute(statement, params.as_slice())
            .await
            .map_err(|e| PgError::QueryFailed(format!("{:?}", e)))?;

        Ok(nrow)
    }

    async fn query(
        &mut self,
        address: &str,
        statement: &str,
        params: Vec<ParameterValue<'_>>,
    ) -> Result<RowSet, PgError> {
        let (client, connection) = tokio_postgres::connect(address, NoTls)
            .await
            .map_err(|e| PgError::ConnectionFailed(format!("{:?}", e)))?;

        spawn(connection);

        let params: Vec<&(dyn ToSql + Sync)> = params
            .iter()
            .map(to_sql_parameter)
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(|e| PgError::BadParameter(format!("{:?}", e)))?;

        let results = client
            .query(statement, params.as_slice())
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
}

const DB_NULL: Option<i32> = None;

fn to_sql_parameter<'a>(value: &'a ParameterValue) -> anyhow::Result<&'a (dyn ToSql + Sync)> {
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
        Type::TEXT => DbDataType::Str,
        Type::VARCHAR => DbDataType::Str,
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
        &Type::TEXT => {
            let value: Option<&str> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Str(v.to_owned()),
                None => DbValue::DbNull,
            }
        }
        &Type::VARCHAR => {
            let value: Option<&str> = row.try_get(index)?;
            match value {
                Some(v) => DbValue::Str(v.to_owned()),
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

fn spawn(connection: Connection<Socket, NoTlsStream>) {
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::warn!("Postgres connection error: {}", e);
        }
    });
}
