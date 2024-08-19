mod host;

use anyhow::{Context, Result};
use mysql_async::{consts::ColumnType, from_value_opt, prelude::*, Opts, OptsBuilder, SslOpts};
use spin_core::async_trait;
use spin_core::wasmtime::component::Resource;
use spin_factor_outbound_networking::{OutboundAllowedHosts, OutboundNetworkingFactor};
use spin_factors::{Factor, InitContext, RuntimeFactors, SelfInstanceBuilder};
use spin_world::v1::mysql as v1;
use spin_world::v2::mysql::{self as v2, Connection};
use spin_world::v2::rdbms_types as v2_types;
use spin_world::v2::rdbms_types::{Column, DbDataType, DbValue, ParameterValue};
use std::sync::Arc;
use tracing::{instrument, Level};
use url::Url;

pub struct OutboundMysqlFactor {}

impl Factor for OutboundMysqlFactor {
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(&mut self, mut ctx: InitContext<T, Self>) -> anyhow::Result<()> {
        ctx.link_bindings(v1::add_to_linker)?;
        ctx.link_bindings(v2::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        ctx: spin_factors::ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        Ok(())
    }

    fn prepare<T: spin_factors::RuntimeFactors>(
        &self,
        ctx: spin_factors::PrepareContext<Self>,
        builders: &mut spin_factors::InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder> {
        let allowed_hosts = builders
            .get_mut::<OutboundNetworkingFactor>()?
            .allowed_hosts();
        Ok(InstanceState {
            allowed_hosts,
            connections: Default::default(),
        })
    }
}

pub struct InstanceState {
    allowed_hosts: OutboundAllowedHosts,
    connections: table::Table<mysql_async::Conn>,
}

impl SelfInstanceBuilder for InstanceState {}
