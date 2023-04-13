use std::{
    io::Cursor,
    path::PathBuf,
    time::{Duration, Instant},
};

use spin_core::{
    Component, Config, Engine, HostComponent, I32Exit, Store, StoreBuilder, Trap, Wasi,
};
use tempfile::TempDir;
use tokio::fs;

#[tokio::test(flavor = "multi_thread")]
async fn test_stdio() {
    let stdout = run_core_wasi_test(["echo"], |store_builder| {
        store_builder.stdin_pipe(Cursor::new(b"DATA"));
    })
    .await
    .unwrap();

    assert_eq!(stdout, "DATA");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_read_only_preopened_dir() {
    let filename = "test_file";
    let tempdir = TempDir::new().unwrap();
    std::fs::write(tempdir.path().join(filename), "x").unwrap();

    run_core_wasi_test(["read", filename], |store_builder| {
        store_builder
            .read_only_preopened_dir(&tempdir, "/".into())
            .unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_read_only_preopened_dir_write_fails() {
    let filename = "test_file";
    let tempdir = TempDir::new().unwrap();
    std::fs::write(tempdir.path().join(filename), "x").unwrap();

    let err = run_core_wasi_test(["write", filename], |store_builder| {
        store_builder
            .read_only_preopened_dir(&tempdir, "/".into())
            .unwrap();
    })
    .await
    .unwrap_err();
    let trap = err.downcast::<I32Exit>().expect("trap");
    assert_eq!(trap.0, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_read_write_preopened_dir() {
    let filename = "test_file";
    let tempdir = TempDir::new().unwrap();

    run_core_wasi_test(["write", filename], |store_builder| {
        store_builder
            .read_write_preopened_dir(&tempdir, "/".into())
            .unwrap();
    })
    .await
    .unwrap();

    let content = std::fs::read(tempdir.path().join(filename)).unwrap();
    assert_eq!(content, b"content");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_max_memory_size_obeyed() {
    let max = 10_000_000;
    let alloc = max / 10;
    run_core_wasi_test(["alloc", &format!("{alloc}")], |store_builder| {
        store_builder.max_memory_size(max);
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_max_memory_size_violated() {
    let max = 10_000_000;
    let alloc = max * 2;
    let err = run_core_wasi_test(["alloc", &format!("{alloc}")], |store_builder| {
        store_builder.max_memory_size(max);
    })
    .await
    .unwrap_err();
    let trap = err.downcast::<I32Exit>().expect("trap");
    assert_eq!(trap.0, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_set_deadline_obeyed() {
    run_core_wasi_test_engine(
        &test_engine(),
        ["sleep", "20"],
        |_| {},
        |store| {
            store.set_deadline(Instant::now() + Duration::from_millis(1000));
        },
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_set_deadline_violated() {
    let err = run_core_wasi_test_engine(
        &test_engine(),
        ["sleep", "100"],
        |_| {},
        |store| {
            store.set_deadline(Instant::now() + Duration::from_millis(10));
        },
    )
    .await
    .unwrap_err();
    let trap = err.downcast::<Trap>().expect("trap");
    assert_eq!(trap, Trap::Interrupt);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_host_component() {
    let stdout = run_core_wasi_test(["multiply", "5"], |_| {}).await.unwrap();
    assert_eq!(stdout, "10");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_host_component_data_update() {
    // Need to build Engine separately to get the HostComponentDataHandle
    let mut engine_builder = Engine::builder(&test_config()).unwrap();
    let factor_data_handle = engine_builder
        .add_host_component(MultiplierHostComponent)
        .unwrap();
    let engine: Engine<()> = engine_builder.build();

    let stdout = run_core_wasi_test_engine(
        &engine,
        ["multiply", "5"],
        |store_builder| {
            store_builder
                .host_components_data()
                .set(factor_data_handle, 100);
        },
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(stdout, "500");
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(not(tarpaulin))]
async fn test_panic() {
    let err = run_core_wasi_test(["panic"], |_| {}).await.unwrap_err();
    let trap = err.downcast::<Trap>().expect("trap");
    assert_eq!(trap, Trap::UnreachableCodeReached);
}

fn test_config() -> Config {
    let mut config = Config::default();
    config
        .wasmtime_config()
        .wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config
}

fn test_engine() -> Engine<()> {
    let mut builder = Engine::builder(&test_config()).unwrap();
    builder.add_host_component(MultiplierHostComponent).unwrap();
    builder.build()
}

async fn run_core_wasi_test<'a>(
    args: impl IntoIterator<Item = &'a str>,
    f: impl FnOnce(&mut StoreBuilder),
) -> anyhow::Result<String> {
    run_core_wasi_test_engine(&test_engine(), args, f, |_| {}).await
}

async fn run_core_wasi_test_engine<'a>(
    engine: &Engine<()>,
    args: impl IntoIterator<Item = &'a str>,
    update_store_builder: impl FnOnce(&mut StoreBuilder),
    update_store: impl FnOnce(&mut Store<()>),
) -> anyhow::Result<String> {
    let mut store_builder: StoreBuilder = engine.store_builder(Wasi::new_preview2());
    let mut stdout_buf = store_builder.stdout_buffered()?;
    store_builder.stderr_pipe(TestWriter);
    store_builder.args(args)?;

    update_store_builder(&mut store_builder);

    let mut store = store_builder.build()?;
    let module_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-programs/core-wasi-test.wasm");
    let component = spin_componentize::componentize_command(&fs::read(module_path).await?)?;
    let component = Component::new(engine.as_ref(), &component)?;
    let instance_pre = engine.instantiate_pre(&component)?;
    let instance = instance_pre.instantiate_async(&mut store).await?;
    let func = instance.get_typed_func::<(), (Result<(), ()>,)>(&mut store, "main")?;

    update_store(&mut store);

    func.call_async(&mut store, ())
        .await?
        .0
        .map_err(|()| anyhow::anyhow!("command failed"))?;

    let stdout = String::from_utf8(stdout_buf.take())?.trim_end().into();
    Ok(stdout)
}

// Simple test HostComponent; multiplies the input by the configured factor
#[derive(Clone)]
struct MultiplierHostComponent;

impl HostComponent for MultiplierHostComponent {
    type Data = i32;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        // NOTE: we're trying to avoid wit-bindgen because a git dependency
        // would make this crate unpublishable on crates.io
        linker.instance("imports")?.func_wrap_async(
            "multiply",
            move |mut caller, (input,): (i32,)| {
                Box::new(async move {
                    let &mut factor = get(caller.data_mut());
                    let output = factor * input;
                    Ok((output,))
                })
            },
        )?;
        Ok(())
    }

    fn build_data(&self) -> Self::Data {
        2
    }
}

// Write with `print!`, required for test output capture
#[derive(Copy, Clone)]
struct TestWriter;

impl std::io::Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        print!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
