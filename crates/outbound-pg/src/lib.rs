use outbound_pg::*;
use postgres::{Client, NoTls, types::ToSql};

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
    fn execute(&mut self, statement: &str, params: Vec<&str>) -> Result<u64, Error> {
        let mut client = Client::connect("host=localhost user=postgres password=123 dbname=postgres", NoTls)
                        .map_err(|_| Error::Error)?;

        let params: Vec<&(dyn ToSql + Sync)> = params.iter().map(|item| item as &(dyn ToSql + Sync)).collect();

        let nrow = client
            .execute(statement, params.as_slice())
            .map_err(|_| Error::Error)?;
        
        Ok(nrow)
    }
    
    fn query(&mut self, statement: &str, params: Vec<&str>) -> Result<Vec<Vec<Payload>>, Error> {
        let mut client = Client::connect("host=localhost user=postgres password=123 dbname=postgres", NoTls)
                        .map_err(|_| Error::Error)?;
        
        let params: Vec<&(dyn ToSql + Sync)> = params.iter().map(|item| item as &(dyn ToSql + Sync)).collect();

        let results = client
            .query(statement, params.as_slice())
            .map_err(|_| Error::Error)?;
        
        let mut output: Vec<Vec<Payload>> = Vec::new();
        for row in results {
            let ncol = row.len();
            let mut row_vec = Vec::new();
            for i in 0..ncol {
                let col_payload: &str = row.get(i);
                let col_payload: Payload = col_payload.as_bytes().to_vec();
                row_vec.push(col_payload);
            }
            if row_vec.len() > 0 {
                output.push(row_vec);
            }
        }
            
        Ok(output)
    }

}
