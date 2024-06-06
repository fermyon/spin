pub mod preview1;

use std::{future::Future, net::SocketAddr, path::Path};

use spin_factors::{
    anyhow, AppComponent, Factor, InitContext, InstancePreparers, PrepareContext, RuntimeFactors,
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
    type AppState = ();
    type InstancePreparer = InstancePreparer;
    type InstanceState = InstanceState;

    fn init<Factors: RuntimeFactors>(
        &mut self,
        mut ctx: InitContext<Factors, Self>,
    ) -> anyhow::Result<()> {
        fn type_annotate<T, F>(f: F) -> F
        where
            F: Fn(&mut T) -> &mut dyn WasiView,
        {
            f
        }
        let get_data = ctx.get_data_fn();
        let closure = type_annotate(move |data| get_data(data) as &mut dyn WasiView);
        if let Some(linker) = ctx.linker() {
            use wasmtime_wasi::bindings;
            bindings::clocks::wall_clock::add_to_linker_get_host(linker, closure)?;
            bindings::clocks::monotonic_clock::add_to_linker_get_host(linker, closure)?;
            bindings::filesystem::types::add_to_linker_get_host(linker, closure)?;
            bindings::filesystem::preopens::add_to_linker_get_host(linker, closure)?;
            bindings::io::error::add_to_linker_get_host(linker, closure)?;
            bindings::io::poll::add_to_linker_get_host(linker, closure)?;
            bindings::io::streams::add_to_linker_get_host(linker, closure)?;
            bindings::random::random::add_to_linker_get_host(linker, closure)?;
            bindings::random::insecure::add_to_linker_get_host(linker, closure)?;
            bindings::random::insecure_seed::add_to_linker_get_host(linker, closure)?;
            bindings::cli::exit::add_to_linker_get_host(linker, closure)?;
            bindings::cli::environment::add_to_linker_get_host(linker, closure)?;
            bindings::cli::stdin::add_to_linker_get_host(linker, closure)?;
            bindings::cli::stdout::add_to_linker_get_host(linker, closure)?;
            bindings::cli::stderr::add_to_linker_get_host(linker, closure)?;
            bindings::cli::terminal_input::add_to_linker_get_host(linker, closure)?;
            bindings::cli::terminal_output::add_to_linker_get_host(linker, closure)?;
            bindings::cli::terminal_stdin::add_to_linker_get_host(linker, closure)?;
            bindings::cli::terminal_stdout::add_to_linker_get_host(linker, closure)?;
            bindings::cli::terminal_stderr::add_to_linker_get_host(linker, closure)?;
            bindings::sockets::tcp::add_to_linker_get_host(linker, closure)?;
            bindings::sockets::tcp_create_socket::add_to_linker_get_host(linker, closure)?;
            bindings::sockets::udp::add_to_linker_get_host(linker, closure)?;
            bindings::sockets::udp_create_socket::add_to_linker_get_host(linker, closure)?;
            bindings::sockets::instance_network::add_to_linker_get_host(linker, closure)?;
            bindings::sockets::network::add_to_linker_get_host(linker, closure)?;
            bindings::sockets::ip_name_lookup::add_to_linker_get_host(linker, closure)?;
        }
        Ok(())
    }

    fn create_preparer<T: RuntimeFactors>(
        ctx: PrepareContext<Self>,
        _preparers: InstancePreparers<T>,
    ) -> anyhow::Result<Self::InstancePreparer> {
        let mut wasi_ctx = WasiCtxBuilder::new();

        // Apply environment variables
        for (key, val) in ctx.app_component().environment() {
            wasi_ctx.env(key, val);
        }

        // Mount files
        let mount_ctx = MountFilesContext {
            wasi_ctx: &mut wasi_ctx,
        };
        ctx.factor()
            .files_mounter
            .mount_files(ctx.app_component(), mount_ctx)?;

        Ok(InstancePreparer { wasi_ctx })
    }

    fn prepare(&self, preparer: InstancePreparer) -> anyhow::Result<InstanceState> {
        let InstancePreparer { mut wasi_ctx } = preparer;
        Ok(InstanceState {
            ctx: wasi_ctx.build(),
            table: Default::default(),
        })
    }
}

pub trait FilesMounter {
    fn mount_files(
        &self,
        app_component: &AppComponent,
        ctx: MountFilesContext,
    ) -> anyhow::Result<()>;
}

pub struct DummyFilesMounter;

impl FilesMounter for DummyFilesMounter {
    fn mount_files(
        &self,
        app_component: &AppComponent,
        _ctx: MountFilesContext,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
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
    ) -> anyhow::Result<()> {
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
}

impl Default for InstancePreparer {
    fn default() -> Self {
        Self {
            wasi_ctx: WasiCtxBuilder::new(),
        }
    }
}

impl InstancePreparer {
    pub fn outbound_socket_addr_check<F, Fut>(&mut self, check: F)
    where
        F: Fn(SocketAddr) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = bool> + Send + Sync,
    {
        self.wasi_ctx.socket_addr_check(move |addr, addr_use| {
            let check = check.clone();
            Box::pin(async move {
                match addr_use {
                    wasmtime_wasi::SocketAddrUse::TcpBind => false,
                    wasmtime_wasi::SocketAddrUse::TcpConnect
                    | wasmtime_wasi::SocketAddrUse::UdpBind
                    | wasmtime_wasi::SocketAddrUse::UdpConnect
                    | wasmtime_wasi::SocketAddrUse::UdpOutgoingDatagram => check(addr).await,
                }
            })
        });
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
