pub mod preview1;

use spin_factors::{Factor, FactorBuilder, InitContext, PrepareContext, Result, SpinFactors};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

pub struct WasiFactor;

impl Factor for WasiFactor {
    type Builder = Builder;
    type Data = Data;

    fn init<Factors: SpinFactors>(&mut self, mut ctx: InitContext<Factors, Self>) -> Result<()> {
        use wasmtime_wasi::bindings;
        ctx.link_bindings(bindings::clocks::wall_clock::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::clocks::monotonic_clock::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::filesystem::types::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::filesystem::preopens::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::io::error::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::io::poll::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::io::streams::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::random::random::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::random::insecure::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::random::insecure_seed::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::exit::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::environment::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::stdin::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::stdout::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::stderr::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::terminal_input::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::terminal_output::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::terminal_stdin::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::terminal_stdout::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::cli::terminal_stderr::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::sockets::tcp::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::sockets::tcp_create_socket::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::sockets::udp::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::sockets::udp_create_socket::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::sockets::instance_network::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::sockets::network::add_to_linker_get_host)?;
        ctx.link_bindings(bindings::sockets::ip_name_lookup::add_to_linker_get_host)?;
        Ok(())
    }
}

pub struct Builder {
    wasi_ctx: WasiCtxBuilder,
}

impl FactorBuilder<WasiFactor> for Builder {
    fn prepare<Factors: SpinFactors>(
        _factor: &WasiFactor,
        _ctx: PrepareContext<Factors>,
    ) -> Result<Self> {
        Ok(Self {
            wasi_ctx: WasiCtxBuilder::new(),
        })
    }

    fn build(mut self) -> Result<Data> {
        Ok(Data {
            ctx: self.wasi_ctx.build(),
            table: Default::default(),
        })
    }
}

pub struct Data {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for Data {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
