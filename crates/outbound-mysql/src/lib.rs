use anyhow::Result;
pub use mysql::add_to_linker;
use mysql_async::{consts::ColumnType, from_value_opt, prelude::*, Opts, OptsBuilder, SslOpts};
use spin_core::{
    async_trait,
    mysql::{self, MysqlError},
    rdbms_types::{Column, DbDataType, DbValue, ParameterValue, RowSet},
    HostComponent,
};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

/// A simple implementation to support outbound mysql connection
#[derive(Default)]
pub struct OutboundMysql {
    pub connections: HashMap<String, mysql_async::Conn>,
}

impl HostComponent for OutboundMysql {
    type Data = Self;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        mysql::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

#[async_trait]
impl mysql::Host for OutboundMysql {
    async fn execute(
        &mut self,
        address: String,
        statement: String,
        params: Vec<ParameterValue>,
    ) -> Result<Result<(), MysqlError>> {
        Ok(async {
            let db_params = params
                .iter()
                .map(to_sql_parameter)
                .collect::<anyhow::Result<Vec<_>>>()
                .map_err(|e| MysqlError::QueryFailed(format!("{:?}", e)))?;

            let parameters = mysql_async::Params::Positional(db_params);

            self.get_conn(&address)
                .await
                .map_err(|e| MysqlError::ConnectionFailed(format!("{:?}", e)))?
                .exec_batch(&statement, &[parameters])
                .await
                .map_err(|e| MysqlError::QueryFailed(format!("{:?}", e)))?;

            Ok(())
        }
        .await)
    }

    async fn query(
        &mut self,
        address: String,
        statement: String,
        params: Vec<ParameterValue>,
    ) -> Result<Result<RowSet, MysqlError>> {
        Ok(async {
            let db_params = params
                .iter()
                .map(to_sql_parameter)
                .collect::<anyhow::Result<Vec<_>>>()
                .map_err(|e| MysqlError::QueryFailed(format!("{:?}", e)))?;

            let parameters = mysql_async::Params::Positional(db_params);

            let mut query_result = self
                .get_conn(&address)
                .await
                .map_err(|e| MysqlError::ConnectionFailed(format!("{:?}", e)))?
                .exec_iter(&statement, parameters)
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
        .await)
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
    let column_type = column.column_type();

    if column_type.is_numeric_type() {
        convert_numeric_type(column)
    } else if column_type.is_character_type() {
        convert_character_type(column)
    } else {
        DbDataType::Other
    }
}

fn convert_character_type(column: &mysql_async::Column) -> DbDataType {
    match (column.column_type(), is_binary(column)) {
        (ColumnType::MYSQL_TYPE_BLOB, false) => DbDataType::Str, // TEXT type
        (ColumnType::MYSQL_TYPE_BLOB, _) => DbDataType::Binary,
        (ColumnType::MYSQL_TYPE_LONG_BLOB, _) => DbDataType::Binary,
        (ColumnType::MYSQL_TYPE_MEDIUM_BLOB, _) => DbDataType::Binary,
        (ColumnType::MYSQL_TYPE_STRING, true) => DbDataType::Binary, // BINARY type
        (ColumnType::MYSQL_TYPE_STRING, _) => DbDataType::Str,
        (ColumnType::MYSQL_TYPE_VAR_STRING, true) => DbDataType::Binary, // VARBINARY type
        (ColumnType::MYSQL_TYPE_VAR_STRING, _) => DbDataType::Str,
        (_, _) => DbDataType::Other,
    }
}

fn convert_numeric_type(column: &mysql_async::Column) -> DbDataType {
    match (column.column_type(), is_signed(column)) {
        (ColumnType::MYSQL_TYPE_DOUBLE, _) => DbDataType::Floating64,
        (ColumnType::MYSQL_TYPE_FLOAT, _) => DbDataType::Floating32,
        (ColumnType::MYSQL_TYPE_INT24, true) => DbDataType::Int32,
        (ColumnType::MYSQL_TYPE_INT24, false) => DbDataType::Uint32,
        (ColumnType::MYSQL_TYPE_LONG, true) => DbDataType::Int32,
        (ColumnType::MYSQL_TYPE_LONG, false) => DbDataType::Uint32,
        (ColumnType::MYSQL_TYPE_LONGLONG, true) => DbDataType::Int64,
        (ColumnType::MYSQL_TYPE_LONGLONG, false) => DbDataType::Uint64,
        (ColumnType::MYSQL_TYPE_SHORT, true) => DbDataType::Int16,
        (ColumnType::MYSQL_TYPE_SHORT, false) => DbDataType::Uint16,
        (ColumnType::MYSQL_TYPE_TINY, true) => DbDataType::Int8,
        (ColumnType::MYSQL_TYPE_TINY, false) => DbDataType::Uint8,
        (_, _) => DbDataType::Other,
    }
}

fn is_signed(column: &mysql_async::Column) -> bool {
    !column
        .flags()
        .contains(mysql_async::consts::ColumnFlags::UNSIGNED_FLAG)
}

fn is_binary(column: &mysql_async::Column) -> bool {
    column
        .flags()
        .contains(mysql_async::consts::ColumnFlags::BINARY_FLAG)
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

impl OutboundMysql {
    async fn get_conn(&mut self, address: &str) -> anyhow::Result<&mut mysql_async::Conn> {
        let client = match self.connections.entry(address.to_owned()) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => v.insert(build_conn(address).await?),
        };
        Ok(client)
    }
}

async fn build_conn(address: &str) -> Result<mysql_async::Conn, mysql_async::Error> {
    tracing::log::debug!("Build new connection: {}", address);

    let opts = build_opts(address)?;

    let connection_pool = mysql_async::Pool::new(opts);

    connection_pool.get_conn().await
}

fn is_ssl_param(s: &str) -> bool {
    ["ssl-mode", "sslmode"].contains(&s.to_lowercase().as_str())
}

/// The mysql_async crate blows up if you pass it an SSL parameter and doesn't support SSL opts properly. This function
/// is a workaround to manually set SSL opts if the user requests them.
///
/// We only support ssl-mode in the query as per
/// https://dev.mysql.com/doc/connector-j/8.0/en/connector-j-connp-props-security.html#cj-conn-prop_sslMode.
///
/// An issue has been filed in the upstream repository https://github.com/blackbeam/mysql_async/issues/225.
fn build_opts(address: &str) -> Result<Opts, mysql_async::Error> {
    let url = Url::parse(address)?;

    let use_ssl = url
        .query_pairs()
        .any(|(k, v)| is_ssl_param(&k) && v.to_lowercase() != "disabled");

    let query_without_ssl: Vec<(_, _)> = url
        .query_pairs()
        .filter(|(k, _v)| !is_ssl_param(k))
        .collect();
    let mut cleaned_url = url.clone();
    cleaned_url.set_query(None);
    cleaned_url
        .query_pairs_mut()
        .extend_pairs(query_without_ssl);

    Ok(OptsBuilder::from_opts(cleaned_url.as_str())
        .ssl_opts(if use_ssl {
            Some(SslOpts::default())
        } else {
            None
        })
        .into())
}

fn convert_value_to<T: FromValue>(value: mysql_async::Value) -> Result<T, MysqlError> {
    from_value_opt::<T>(value).map_err(|e| MysqlError::ValueConversionFailed(format!("{}", e)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mysql_address_without_ssl_mode() {
        assert!(build_opts("mysql://myuser:password@127.0.0.1/db")
            .unwrap()
            .ssl_opts()
            .is_none())
    }

    #[test]
    fn test_mysql_address_with_ssl_mode_disabled() {
        assert!(
            build_opts("mysql://myuser:password@127.0.0.1/db?ssl-mode=DISABLED")
                .unwrap()
                .ssl_opts()
                .is_none()
        )
    }

    #[test]
    fn test_mysql_address_with_ssl_mode_verify_ca() {
        assert!(
            build_opts("mysql://myuser:password@127.0.0.1/db?sslMode=VERIFY_CA")
                .unwrap()
                .ssl_opts()
                .is_some()
        )
    }

    #[test]
    fn test_mysql_address_with_more_to_query() {
        let address = "mysql://myuser:password@127.0.0.1/db?SsLmOdE=VERIFY_CA&pool_max=10";
        assert!(build_opts(address).unwrap().ssl_opts().is_some());
        assert_eq!(
            build_opts(address).unwrap().pool_opts().constraints().max(),
            10
        )
    }
}
