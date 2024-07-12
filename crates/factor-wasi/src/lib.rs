use std::{future::Future, net::SocketAddr, path::Path};

use spin_factors::{
    anyhow, AppComponent, Factor, FactorInstanceBuilder, InitContext, InstanceBuilders,
    PrepareContext, RuntimeFactors,
};
use tokio::io::{AsyncRead, AsyncWrite};
use wasmtime_wasi::{
    pipe::{AsyncReadStream, AsyncWriteStream},
    AsyncStdinStream, AsyncStdoutStream, DirPerms, FilePerms, ResourceTable, StdinStream,
    StdoutStream, WasiCtx, WasiCtxBuilder, WasiImpl, WasiView,
};

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
    type RuntimeConfig = ();
    type AppState = ();
    type InstanceBuilder = InstanceBuilder;

    fn init<Factors: RuntimeFactors>(
        &mut self,
        mut ctx: InitContext<Factors, Self>,
    ) -> anyhow::Result<()> {
        fn type_annotate<T, U: WasiView, F>(f: F) -> F
        where
            F: Fn(&mut T) -> WasiImpl<&mut U>,
        {
            f
        }
        let get_data = ctx.get_data_fn();
        let closure = type_annotate(move |data| WasiImpl(get_data(data)));
        let linker = ctx.linker();
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
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        _ctx: spin_factors::ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        Ok(())
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        ctx: PrepareContext<Self>,
        _builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<InstanceBuilder> {
        let mut wasi_ctx = WasiCtxBuilder::new();

        // Mount files
        let mount_ctx = MountFilesContext { ctx: &mut wasi_ctx };
        self.files_mounter
            .mount_files(ctx.app_component(), mount_ctx)?;

        let mut builder = InstanceBuilder { ctx: wasi_ctx };

        // Apply environment variables
        builder.env(ctx.app_component().environment());

        Ok(builder)
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
    ctx: &'a mut WasiCtxBuilder,
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
        self.ctx
            .preopened_dir(host_path, guest_path, dir_perms, file_perms)?;
        Ok(())
    }
}

pub struct InstanceBuilder {
    ctx: WasiCtxBuilder,
}

impl InstanceBuilder {
    /// Sets the WASI `stdin` descriptor to the given [`StdinStream`].
    pub fn stdin(&mut self, stdin: impl StdinStream + 'static) {
        self.ctx.stdin(stdin);
    }

    /// Sets the WASI `stdin` descriptor to the given [`AsyncRead`]er.
    pub fn stdin_pipe(&mut self, r: impl AsyncRead + Send + Unpin + 'static) {
        self.stdin(AsyncStdinStream::new(AsyncReadStream::new(r)));
    }

    /// Sets the WASI `stdout` descriptor to the given [`StdoutStream`].
    pub fn stdout(&mut self, stdout: impl StdoutStream + 'static) {
        self.ctx.stdout(stdout);
    }

    /// Sets the WASI `stdout` descriptor to the given [`AsyncWrite`]r.
    pub fn stdout_pipe(&mut self, w: impl AsyncWrite + Send + Unpin + 'static) {
        self.stdout(AsyncStdoutStream::new(AsyncWriteStream::new(
            1024 * 1024,
            w,
        )));
    }

    /// Sets the WASI `stderr` descriptor to the given [`StdoutStream`].
    pub fn stderr(&mut self, stderr: impl StdoutStream + 'static) {
        self.ctx.stderr(stderr);
    }

    /// Sets the WASI `stderr` descriptor to the given [`AsyncWrite`]r.
    pub fn stderr_pipe(&mut self, w: impl AsyncWrite + Send + Unpin + 'static) {
        self.stderr(AsyncStdoutStream::new(AsyncWriteStream::new(
            1024 * 1024,
            w,
        )));
    }

    /// Appends the given strings to the WASI 'args'.
    pub fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<str>>) {
        for arg in args {
            self.ctx.arg(arg);
        }
    }

    /// Sets the given key/value string entries on the WASI 'env'.
    pub fn env(&mut self, vars: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>) {
        for (k, v) in vars {
            self.ctx.env(k, v);
        }
    }

    /// "Mounts" the given `host_path` into the WASI filesystem at the given
    /// `guest_path`.
    pub fn preopened_dir(
        &mut self,
        host_path: impl AsRef<Path>,
        guest_path: impl AsRef<str>,
        writable: bool,
    ) -> anyhow::Result<()> {
        let (dir_perms, file_perms) = if writable {
            (DirPerms::all(), FilePerms::all())
        } else {
            (DirPerms::READ, FilePerms::READ)
        };
        self.ctx
            .preopened_dir(host_path, guest_path, dir_perms, file_perms)?;
        Ok(())
    }
}

impl FactorInstanceBuilder for InstanceBuilder {
    type InstanceState = InstanceState;

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        let InstanceBuilder { ctx: mut wasi_ctx } = self;
        Ok(InstanceState {
            ctx: wasi_ctx.build(),
            table: Default::default(),
        })
    }
}

impl InstanceBuilder {
    pub fn outbound_socket_addr_check<F, Fut>(&mut self, check: F)
    where
        F: Fn(SocketAddr) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = bool> + Send + Sync,
    {
        self.ctx.socket_addr_check(move |addr, addr_use| {
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

impl InstanceState {
    pub fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl WasiView for InstanceState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}