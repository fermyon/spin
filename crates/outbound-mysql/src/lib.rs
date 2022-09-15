use mysql_async::consts::ColumnType;
use mysql_async::{from_value_opt, prelude::*};
pub use outbound_mysql::add_to_linker;
use spin_core::HostComponent;
use std::sync::Arc;
use wit_bindgen_wasmtime::async_trait;

wit_bindgen_wasmtime::export!({paths: ["../../wit/ephemeral/outbound-mysql.wit"], async: *});
use outbound_mysql::*;

/// A simple implementation to support outbound mysql connection
#[derive(Default, Clone)]
pub struct OutboundMysql;

impl HostComponent for OutboundMysql {
    type Data = Self;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        outbound_mysql::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

#[async_trait]
impl outbound_mysql::OutboundMysql for OutboundMysql {
    async fn execute(
        &mut self,
        address: &str,
        statement: &str,
        params: Vec<ParameterValue<'_>>,
    ) -> Result<(), MysqlError> {
        let connection_pool = mysql_async::Pool::new(address);
        let mut connection = connection_pool
            .get_conn()
            .await
            .map_err(|e| MysqlError::ConnectionFailed(format!("{:?}", e)))?;

        let db_params = params
            .iter()
            .map(to_sql_parameter)
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(|e| MysqlError::QueryFailed(format!("{:?}", e)))?;

        let parameters = mysql_async::Params::Positional(db_params);
        connection
            .exec_batch(statement, &[parameters])
            .await
            .map_err(|e| MysqlError::QueryFailed(format!("{:?}", e)))?;

        Ok(())
    }

    async fn query(
        &mut self,
        address: &str,
        statement: &str,
        params: Vec<ParameterValue<'_>>,
    ) -> Result<RowSet, MysqlError> {
        let connection_pool = mysql_async::Pool::new(address);
        let mut connection = connection_pool
            .get_conn()
            .await
            .map_err(|e| MysqlError::ConnectionFailed(format!("{:?}", e)))?;

        let db_params = params
            .iter()
            .map(to_sql_parameter)
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(|e| MysqlError::QueryFailed(format!("{:?}", e)))?;

        let parameters = mysql_async::Params::Positional(db_params);
        let mut query_result = connection
            .exec_iter(statement, parameters)
            .await
            .map_err(|e| MysqlError::QueryFailed(format!("{:?}", e)))?;

        // We have to get these before collect() destroys them
        let columns = convert_columns(query_result.columns());

        match query_result.collect::<mysql_async::Row>().await {
            Err(e) => Err(MysqlError::OtherError(format!("{:?}", e))),
            Ok(result_set) => {
                let rows = result_set
                    .into_iter()
                    .map(|row| convert_row(row, &columns))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| MysqlError::QueryFailed(format!("{:?}", e)))?;

                Ok(RowSet { columns, rows })
            }
        }
    }
}

fn to_sql_parameter(value: &ParameterValue) -> anyhow::Result<mysql_async::Value> {
    match value {
        ParameterValue::Boolean(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Int32(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Int64(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Int8(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Int16(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Floating32(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Floating64(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Uint8(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Uint16(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Uint32(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Uint64(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Str(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::Binary(v) => Ok(mysql_async::Value::from(v)),
        ParameterValue::DbNull => Ok(mysql_async::Value::NULL),
    }
}

fn convert_columns(columns: Option<Arc<[mysql_async::Column]>>) -> Vec<Column> {
    match columns {
        Some(columns) => columns.iter().map(convert_column).collect(),
        None => vec![],
    }
}

fn convert_column(column: &mysql_async::Column) -> Column {
    let name = column.name_str().to_string();
    let data_type = convert_data_type(column);

    Column { name, data_type }
}

fn convert_data_type(column: &mysql_async::Column) -> DbDataType {
    match (column.column_type(), is_signed(column)) {
        (ColumnType::MYSQL_TYPE_BIT, _) => DbDataType::Boolean,
        (ColumnType::MYSQL_TYPE_BLOB, _) => DbDataType::Binary,
        (ColumnType::MYSQL_TYPE_DOUBLE, _) => DbDataType::Floating64,
        (ColumnType::MYSQL_TYPE_FLOAT, _) => DbDataType::Floating32,
        (ColumnType::MYSQL_TYPE_LONG, true) => DbDataType::Int32,
        (ColumnType::MYSQL_TYPE_LONG, false) => DbDataType::Uint32,
        (ColumnType::MYSQL_TYPE_LONGLONG, true) => DbDataType::Int64,
        (ColumnType::MYSQL_TYPE_LONGLONG, false) => DbDataType::Uint64,
        (ColumnType::MYSQL_TYPE_LONG_BLOB, _) => DbDataType::Binary,
        (ColumnType::MYSQL_TYPE_MEDIUM_BLOB, _) => DbDataType::Binary,
        (ColumnType::MYSQL_TYPE_SHORT, true) => DbDataType::Int16,
        (ColumnType::MYSQL_TYPE_SHORT, false) => DbDataType::Uint16,
        (ColumnType::MYSQL_TYPE_STRING, _) => DbDataType::Str,
        (ColumnType::MYSQL_TYPE_TINY, true) => DbDataType::Int8,
        (ColumnType::MYSQL_TYPE_TINY, false) => DbDataType::Uint8,
        (ColumnType::MYSQL_TYPE_VARCHAR, _) => DbDataType::Str,
        (ColumnType::MYSQL_TYPE_VAR_STRING, _) => DbDataType::Str,
        (_, _) => DbDataType::Other,
    }
}

fn is_signed(column: &mysql_async::Column) -> bool {
    !column
        .flags()
        .contains(mysql_async::consts::ColumnFlags::UNSIGNED_FLAG)
}

fn convert_row(mut row: mysql_async::Row, columns: &[Column]) -> Result<Vec<DbValue>, MysqlError> {
    let mut result = Vec::with_capacity(row.len());
    for index in 0..row.len() {
        result.push(convert_entry(&mut row, index, columns)?);
    }
    Ok(result)
}

fn convert_entry(
    row: &mut mysql_async::Row,
    index: usize,
    columns: &[Column],
) -> Result<DbValue, MysqlError> {
    match (row.take(index), columns.get(index)) {
        (None, _) => Ok(DbValue::DbNull), // TODO: is this right or is this an "index out of range" thing
        (_, None) => Err(MysqlError::OtherError(format!(
            "Can't get column at index {}",
            index
        ))),
        (Some(mysql_async::Value::NULL), _) => Ok(DbValue::DbNull),
        (Some(value), Some(column)) => convert_value(value, column),
    }
}

fn convert_value(value: mysql_async::Value, column: &Column) -> Result<DbValue, MysqlError> {
    match column.data_type {
        DbDataType::Binary => convert_value_to::<Vec<u8>>(value).map(DbValue::Binary),
        DbDataType::Boolean => convert_value_to::<bool>(value).map(DbValue::Boolean),
        DbDataType::Floating32 => convert_value_to::<f32>(value).map(DbValue::Floating32),
        DbDataType::Floating64 => convert_value_to::<f64>(value).map(DbValue::Floating64),
        DbDataType::Int8 => convert_value_to::<i8>(value).map(DbValue::Int8),
        DbDataType::Int16 => convert_value_to::<i16>(value).map(DbValue::Int16),
        DbDataType::Int32 => convert_value_to::<i32>(value).map(DbValue::Int32),
        DbDataType::Int64 => convert_value_to::<i64>(value).map(DbValue::Int64),
        DbDataType::Str => convert_value_to::<String>(value).map(DbValue::Str),
        DbDataType::Uint8 => convert_value_to::<u8>(value).map(DbValue::Uint8),
        DbDataType::Uint16 => convert_value_to::<u16>(value).map(DbValue::Uint16),
        DbDataType::Uint32 => convert_value_to::<u32>(value).map(DbValue::Uint32),
        DbDataType::Uint64 => convert_value_to::<u64>(value).map(DbValue::Uint64),
        DbDataType::Other => Err(MysqlError::ValueConversionFailed(format!(
            "Cannot convert value {:?} in column {} data type {:?}",
            value, column.name, column.data_type
        ))),
    }
}

fn convert_value_to<T: FromValue>(value: mysql_async::Value) -> Result<T, MysqlError> {
    from_value_opt::<T>(value).map_err(|e| MysqlError::ValueConversionFailed(format!("{}", e)))
}
