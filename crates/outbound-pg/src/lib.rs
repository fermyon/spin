use outbound_pg::*;
use postgres::{types::ToSql, types::Type, Client, NoTls, Row};

pub use outbound_pg::add_to_linker;
use spin_engine::{
    host_component::{HostComponent, HostComponentsStateHandle},
    RuntimeContext,
};
use wit_bindgen_wasmtime::wasmtime::Linker;

wit_bindgen_wasmtime::export!("../../wit/ephemeral/outbound-pg.wit");

/// A simple implementation to support outbound pg connection
#[derive(Default, Clone)]
pub struct OutboundPg;

impl HostComponent for OutboundPg {
    type State = Self;

    fn add_to_linker<T>(
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

impl outbound_pg::OutboundPg for OutboundPg {
    fn execute(
        &mut self,
        address: &str,
        statement: &str,
        params: Vec<&str>,
    ) -> Result<u64, PgError> {
        let mut client =
            Client::connect(address, NoTls).map_err(|_| PgError::OtherError("tba".to_owned()))?;

        let params: Vec<&(dyn ToSql + Sync)> = params
            .iter()
            .map(|item| item as &(dyn ToSql + Sync))
            .collect();

        let nrow = client
            .execute(statement, params.as_slice())
            .map_err(|_| PgError::OtherError("tba".to_owned()))?;

        Ok(nrow)
    }

    fn query(
        &mut self,
        address: &str,
        statement: &str,
        params: Vec<&str>,
    ) -> Result<RowSet, PgError> {
        let mut client = Client::connect(address, NoTls)
            .map_err(|e| PgError::ConnectionFailed(format!("{:?}", e)))?;

        let params: Vec<&(dyn ToSql + Sync)> = params
            .iter()
            .map(|item| item as &(dyn ToSql + Sync))
            .collect();

        let results = client
            .query(statement, params.as_slice())
            .map_err(|e| PgError::QueryFailed(format!("{:?}", e)))?;

        let rows = results.iter().map(convert_row).collect();

        Ok(RowSet { rows })
    }
}

fn convert_row(row: &Row) -> Vec<DbValue> {
    let mut result = Vec::with_capacity(row.len());
    for index in 0..row.len() {
        result.push(convert_entry(row, index));
    }
    result
}

fn convert_entry(row: &Row, index: usize) -> DbValue {
    let column = &row.columns()[index];
    match column.type_() {
        &Type::BOOL => {
            let value: Option<bool> = row.get(index);
            match value {
                Some(v) => DbValue::Boolean(v),
                None => DbValue::DbNull,
            }
        }
        &Type::INT4 => {
            let value: Option<i32> = row.get(index);
            match value {
                Some(v) => DbValue::Int32(v),
                None => DbValue::DbNull,
            }
        }
        &Type::INT8 => {
            let value: Option<i64> = row.get(index);
            match value {
                Some(v) => DbValue::Int64(v),
                None => DbValue::DbNull,
            }
        }
        &Type::VARCHAR => {
            let value: Option<&str> = row.get(index);
            match value {
                Some(v) => DbValue::DbString(v.to_owned()),
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
    }
}
