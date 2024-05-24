pub mod preview1;

use std::path::Path;

use anyhow::ensure;
use cap_primitives::{ipnet::IpNet, net::Pool};
use spin_factors::{
    AppComponent, Factor, FactorInstancePreparer, InitContext, PrepareContext, Result, SpinFactors,
};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

pub struct WasiFactor {
    files_mounter: Box<dyn FilesMounter>,
}

impl WasiFactor {
    pub fn new(files_mounter: impl FilesMounter + 'static) -> Self {
        Self {
            files_mounter: Box::new(files_mounter),
        }
    }
}

impl Factor for WasiFactor {
    type AppConfig = ();
    type InstancePreparer = InstancePreparer;
    type InstanceState = InstanceState;

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

pub trait FilesMounter {
    fn mount_files(&self, app_component: &AppComponent, ctx: MountFilesContext) -> Result<()>;
}

pub struct DummyFilesMounter;

impl FilesMounter for DummyFilesMounter {
    fn mount_files(&self, app_component: &AppComponent, _ctx: MountFilesContext) -> Result<()> {
        ensure!(
            app_component.files().next().is_none(),
            "DummyFilesMounter can't actually mount files"
        );
        Ok(())
    }
}

pub struct MountFilesContext<'a> {
    wasi_ctx: &'a mut WasiCtxBuilder,
}

impl<'a> MountFilesContext<'a> {
    pub fn preopened_dir(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: impl AsRef<str>,
        writable: bool,
    ) -> Result<()> {
        use wasmtime_wasi::{DirPerms, FilePerms};
        let (dir_perms, file_perms) = if writable {
            (DirPerms::all(), FilePerms::all())
        } else {
            (DirPerms::READ, FilePerms::READ)
        };
        self.wasi_ctx
            .preopened_dir(host_path, guest_path, dir_perms, file_perms)?;
        Ok(())
    }
}

pub struct InstancePreparer {
    wasi_ctx: WasiCtxBuilder,
    socket_allow_ports: Pool,
}

impl FactorInstancePreparer<WasiFactor> for InstancePreparer {
    // NOTE: Replaces WASI parts of AppComponent::apply_store_config
    fn new<Factors: SpinFactors>(
        factor: &WasiFactor,
        app_component: &AppComponent,
        _ctx: PrepareContext<Factors>,
    ) -> Result<Self> {
        let mut wasi_ctx = WasiCtxBuilder::new();

        // Apply environment variables
        for (key, val) in app_component.environment() {
            wasi_ctx.env(key, val);
        }

        // Mount files
        let mount_ctx = MountFilesContext {
            wasi_ctx: &mut wasi_ctx,
        };
        factor.files_mounter.mount_files(app_component, mount_ctx)?;

        Ok(Self {
            wasi_ctx,
            socket_allow_ports: Default::default(),
        })
    }

    fn prepare(self) -> Result<InstanceState> {
        let Self {
            mut wasi_ctx,
            socket_allow_ports,
        } = self;

        // Enforce socket_allow_ports
        wasi_ctx.socket_addr_check(move |addr, _| socket_allow_ports.check_addr(addr).is_ok());

        Ok(InstanceState {
            ctx: wasi_ctx.build(),
            table: Default::default(),
        })
    }
}

impl InstancePreparer {
    pub fn inherit_network(&mut self) {
        self.wasi_ctx.inherit_network();
    }

    pub fn socket_allow_ports(&mut self, ip_net: IpNet, ports_start: u16, ports_end: Option<u16>) {
        self.socket_allow_ports.insert_ip_net_port_range(
            ip_net,
            ports_start,
            ports_end,
            cap_primitives::ambient_authority(),
        );
    }
}

pub struct InstanceState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for InstanceState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}
