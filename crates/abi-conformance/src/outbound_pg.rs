use super::Context;
use anyhow::{ensure, Result};
use outbound_pg::{Column, DbDataType, DbValue, ParameterValue, PgError, RowSet};
use serde::Serialize;
use std::{collections::HashMap, iter};
use wasmtime::{InstancePre, Store};

pub(super) use outbound_pg::add_to_linker;

/// Report of which outbound PostgreSQL functions a module successfully used, if any
#[derive(Serialize)]
pub struct PgReport {
    /// Result of the PostgreSQL statement execution test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["outbound-pg-execute",
    /// "127.0.0.1", "INSERT INTO foo (x) VALUES ($1)", "int8:42"\] as arguments.  The module should call the
    /// host-implemented `outbound-pg::execute` function with the arguments \["127.0.0.1", "INSERT INTO foo (x)
    /// VALUES ($1)", `\[int8(42)\]`\] and expect `ok(1)` as the result.  The host will assert that said function
    /// is called exactly once with the specified arguments.
    pub execute: Result<(), String>,

    /// Result of the PostgreSQL query execution test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["outbound-pg-query",
    /// "127.0.0.1", "SELECT x FROM foo"\] as arguments.  The module should call the host-implemented
    /// `outbound-pg::execute` function with the arguments \["127.0.0.1", "SELECT x FROM foo"\] and expect `ok({
    /// columns: \[ { name: "x", data_type: int8 } \], rows: \[ \[ int8(42) \] \]})` as the result.  The host will
    /// assert that said function is called exactly once with the specified arguments.
    pub query: Result<(), String>,
}

wit_bindgen_wasmtime::export!("../../wit/ephemeral/outbound-pg.wit");

#[derive(Default)]
pub(super) struct OutboundPg {
    execute_map: HashMap<(String, String, String), u64>,
    query_map: HashMap<(String, String, String), RowSet>,
}

impl outbound_pg::OutboundPg for OutboundPg {
    fn execute(
        &mut self,
        address: &str,
        statement: &str,
        params: Vec<ParameterValue<'_>>,
    ) -> Result<u64, PgError> {
        self.execute_map
            .remove(&(
                address.to_owned(),
                statement.to_owned(),
                format!("{params:?}"),
            ))
            .ok_or_else(|| {
                PgError::OtherError(format!(
                    "expected {:?}, got {:?}",
                    self.execute_map.keys(),
                    iter::once(&(
                        address.to_owned(),
                        statement.to_owned(),
                        format!("{params:?}")
                    ))
                ))
            })
    }

    fn query(
        &mut self,
        address: &str,
        statement: &str,
        params: Vec<ParameterValue<'_>>,
    ) -> Result<RowSet, PgError> {
        self.query_map
            .remove(&(
                address.to_owned(),
                statement.to_owned(),
                format!("{params:?}"),
            ))
            .ok_or_else(|| {
                PgError::OtherError(format!(
                    "expected {:?}, got {:?}",
                    self.query_map.keys(),
                    iter::once(&(
                        address.to_owned(),
                        statement.to_owned(),
                        format!("{params:?}")
                    ))
                ))
            })
    }
}

pub(super) fn test(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<PgReport> {
    Ok(PgReport {
        execute: test_execute(store, pre),
        query: test_query(store, pre),
    })
}

fn test_execute(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<(), String> {
    store.data_mut().outbound_pg.execute_map.insert(
        (
            "127.0.0.1".into(),
            "INSERT INTO foo (x) VALUES ($1)".into(),
            format!("{:?}", vec![ParameterValue::Int8(42)]),
        ),
        1,
    );

    super::run_command(
        store,
        pre,
        &[
            "outbound-pg-execute",
            "127.0.0.1",
            "INSERT INTO foo (x) VALUES ($1)",
            "int8:42",
        ],
        |store| {
            ensure!(
                store.data().outbound_pg.execute_map.is_empty(),
                "expected module to call `outbound-pg::execute` exactly once"
            );

            Ok(())
        },
    )
}

fn test_query(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<(), String> {
    let row_set = RowSet {
        columns: vec![Column {
            name: "x".into(),
            data_type: DbDataType::Int8,
        }],
        rows: vec![vec![DbValue::Int8(42)]],
    };

    store.data_mut().outbound_pg.query_map.insert(
        (
            "127.0.0.1".into(),
            "SELECT x FROM foo".into(),
            format!("{:?}", Vec::<()>::new()),
        ),
        row_set,
    );

    super::run_command(
        store,
        pre,
        &["outbound-pg-query", "127.0.0.1", "SELECT x FROM foo"],
        |store| {
            ensure!(
                store.data().outbound_pg.query_map.is_empty(),
                "expected module to call `outbound-pg::query` exactly once"
            );

            Ok(())
        },
    )
}
