use super::Context;
use anyhow::{ensure, Result};
use cap_std::time::SystemTime as CapStdSystemTime;
use rand::SeedableRng;
use rand_chacha::ChaCha12Core;
use rand_core::block::{BlockRng, BlockRngCore};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::File,
    io::Write,
    ops::Deref,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use wasi_common::{
    pipe::{ReadPipe, WritePipe},
    WasiSystemClock,
};
use wasmtime::{InstancePre, Store};
use wasmtime_wasi::{sync::dir::Dir, Dir as CapStdDir};

/// Report of which WASI functions a module successfully used, if any
///
/// This represents the subset of WASI which is relevant to Spin and does not include e.g. network sockets,
/// filesystem odification, etc.
#[derive(Serialize)]
pub struct WasiReport {
    /// Result of the WASI environment variable test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["wasi-env", "foo"\] as
    /// arguments.  The module should call the host-implemented `wasi_snapshot_preview1::environ_get` function
    /// w`ok("foo=bar")` as the result.  The module should extract the value of the "foo" variable and write the
    /// result to `stdout` as a UTF-8 string.  The host will assert the output matches the expected value.
    pub env: Result<(), String>,

    /// Result of the WASI system clock test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["wasi-epoch"\] as the
    /// argument.  The module should call the host-implemented `wasi_snapshot_preview1::clock_time_get` function
    /// with `realtime` as the clock ID and expect `ok(1663014331719000000)` as the result.  The module should then
    /// divide that value by 1000000 to convert to milliseconds and write the result to `stdout` as a UTF-8 string.
    /// The host will assert the output matches the expected value.
    pub epoch: Result<(), String>,

    /// Result of the WASI system random number generator test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["wasi-random"\] as the
    /// argument.  The module should call the host-implemented `wasi_snapshot_preview1::random_get` function at
    /// least once.  The host will assert that said function was called at least once.
    pub random: Result<(), String>,

    /// Result of the WASI stdio test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["wasi-stdio"\] as the
    /// argument.  The module should call the host-implemented `wasi_snapshot_preview1::fd_read` and
    /// `wasi_snapshot_preview1::fd_write` functions as necessary to read the UTF-8 string "All mimsy were the
    /// borogroves" from `stdin` and write the same string back to `stdout`.  The host will assert that the output
    /// matches the input.
    pub stdio: Result<(), String>,

    /// Result of the WASI filesystem read test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["wasi-read",
    /// "foo.txt"\] as arguments.  The module should call the relevant `wasi_snapshot_preview1` functions to open
    /// the file "foo.txt" in the preopened directory descriptor 3 and read its content, which will be the UTF-8
    /// string "And the mome raths outgrabe".  The module should then write that string to `stdout`.  The host will
    /// assert that the output matches the contents of the file.
    pub read: Result<(), String>,

    /// Result of the WASI filesystem readdir test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["wasi-readdir", "/"\]
    /// as arguments.  The module should call the relevant `wasi_snapshot_preview1` functions to read the contents
    /// of the preopened directory named "/" and write them to `stdout` as comma-delimited, UTF-8-encoded strings
    /// (in arbitrary order), skipping the "." and ".." entries.  The host will assert that the output matches the
    /// contents of the directory: "bar.txt", "baz.txt", and "foo.txt".
    pub readdir: Result<(), String>,

    /// Result of the WASI filesystem stat test
    ///
    /// The guest module should expect a call according to [`super::InvocationStyle`] with \["wasi-stat",
    /// "foo.txt"\] as arguments.  The module should call the relevant `wasi_snapshot_preview1` functions to
    /// retrieve metadata from the file "foo.txt" in the preopened directory descriptor 3.  The module should then
    /// write a UTF-8-encoded string of the form "length:<length>,modified:<modified>" to `stdout`, where
    /// "<length>" is the length of the file and "<modified>" is the last-modified time in milliseconds since 1970
    /// UTC.  The host will assert that the output matches the metdata of the file.
    pub stat: Result<(), String>,
}

pub(super) fn test(store: &mut Store<Context>, pre: &InstancePre<Context>) -> Result<WasiReport> {
    Ok(WasiReport {
        env: {
            let stdout = WritePipe::new_in_memory();
            store.data_mut().wasi.set_stdout(Box::new(stdout.clone()));
            store.data_mut().wasi.push_env("foo", "bar")?;

            super::run_command(store, pre, &["wasi-env", "foo"], move |_| {
                let stdout = String::from_utf8(stdout.try_into_inner().unwrap().into_inner())?;
                ensure!(
                    "bar" == stdout.deref(),
                    "expected module to write \"bar\" to stdout, got {stdout:?}"
                );

                Ok(())
            })
        },

        epoch: {
            const TIME: u64 = 1663014331719;

            struct MyClock;

            impl WasiSystemClock for MyClock {
                fn resolution(&self) -> Duration {
                    Duration::from_millis(1)
                }

                fn now(&self, _precision: Duration) -> CapStdSystemTime {
                    CapStdSystemTime::from_std(
                        SystemTime::UNIX_EPOCH
                            .checked_add(Duration::from_millis(TIME))
                            .unwrap(),
                    )
                }
            }

            let stdout = WritePipe::new_in_memory();
            {
                let context = store.data_mut();
                context.wasi.set_stdout(Box::new(stdout.clone()));
                context.wasi.clocks.system = Box::new(MyClock);
            }

            super::run_command(store, pre, &["wasi-epoch"], move |_| {
                let stdout = String::from_utf8(stdout.try_into_inner().unwrap().into_inner())?;
                ensure!(
                    TIME.to_string() == stdout,
                    "expected module to write {TIME:?} to stdout, got {stdout:?}"
                );

                Ok(())
            })
        },

        random: {
            #[derive(Clone)]
            struct MyRngCore {
                cha_cha_12: ChaCha12Core,
                called: Arc<AtomicBool>,
            }

            impl BlockRngCore for MyRngCore {
                type Item = <ChaCha12Core as BlockRngCore>::Item;
                type Results = <ChaCha12Core as BlockRngCore>::Results;

                fn generate(&mut self, results: &mut Self::Results) {
                    self.called.store(true, Ordering::Relaxed);
                    self.cha_cha_12.generate(results)
                }
            }

            let called = Arc::new(AtomicBool::default());
            store.data_mut().wasi.random = Box::new(BlockRng::new(MyRngCore {
                cha_cha_12: ChaCha12Core::seed_from_u64(42),
                called: called.clone(),
            }));

            super::run_command(store, pre, &["wasi-random"], move |_| {
                ensure!(
                    called.load(Ordering::Relaxed),
                    "expected module to call `wasi_snapshot_preview1::random_get` at least once"
                );

                Ok(())
            })
        },

        stdio: {
            let stdin = ReadPipe::from("All mimsy were the borogroves");
            let stdout = WritePipe::new_in_memory();

            store.data_mut().wasi.set_stdin(Box::new(stdin.clone()));
            store.data_mut().wasi.set_stdout(Box::new(stdout.clone()));

            super::run_command(store, pre, &["wasi-stdio"], move |_| {
                let stdin = stdin.try_into_inner().unwrap().into_inner();
                let stdout = String::from_utf8(stdout.try_into_inner().unwrap().into_inner())?;
                ensure!(
                    stdin == stdout.deref(),
                    "expected module to write {stdin:?} to stdout, got {stdout:?}"
                );

                Ok(())
            })
        },

        read: {
            let stdout = WritePipe::new_in_memory();
            let message = "And the mome raths outgrabe";
            let dir = tempfile::tempdir()?;
            let mut file = File::create(dir.path().join("foo.txt"))?;
            file.write_all(message.as_bytes())?;

            store.data_mut().wasi.set_stdout(Box::new(stdout.clone()));
            add_dir(store, dir.path())?;

            super::run_command(store, pre, &["wasi-read", "foo.txt"], move |_| {
                let stdout = String::from_utf8(stdout.try_into_inner().unwrap().into_inner())?;
                ensure!(
                    message == stdout.deref(),
                    "expected module to write {message:?} to stdout, got {stdout:?}"
                );

                Ok(())
            })
        },

        readdir: {
            let stdout = WritePipe::new_in_memory();
            let dir = tempfile::tempdir()?;

            let names = ["foo.txt", "bar.txt", "baz.txt"];
            for &name in &names {
                File::create(dir.path().join(name))?;
            }

            store.data_mut().wasi.set_stdout(Box::new(stdout.clone()));
            add_dir(store, dir.path())?;

            super::run_command(store, pre, &["wasi-readdir", "/"], move |_| {
                let expected = names.iter().copied().collect::<HashSet<_>>();
                let stdout = String::from_utf8(stdout.try_into_inner().unwrap().into_inner())?;
                let got = stdout.split(',').collect();
                ensure!(
                    expected == got,
                    "expected module to write {expected:?} to stdout (in any order), got {got:?}"
                );

                Ok(())
            })
        },

        stat: {
            let stdout = WritePipe::new_in_memory();
            let message = "O frabjous day! Callooh! Callay!";
            let dir = tempfile::tempdir()?;
            let mut file = File::create(dir.path().join("foo.txt"))?;
            file.write_all(message.as_bytes())?;
            let metadata = file.metadata()?;

            store.data_mut().wasi.set_stdout(Box::new(stdout.clone()));
            add_dir(store, dir.path())?;

            super::run_command(store, pre, &["wasi-stat", "foo.txt"], move |_| {
                let expected = format!(
                    "length:{},modified:{}",
                    metadata.len(),
                    metadata
                        .modified()?
                        .duration_since(SystemTime::UNIX_EPOCH)?
                        .as_millis()
                );
                let got = String::from_utf8(stdout.try_into_inner().unwrap().into_inner())?;

                ensure!(
                    expected == got,
                    "expected module to write {expected:?} to stdout, got {got:?}"
                );

                Ok(())
            })
        },
    })
}

fn add_dir(store: &mut Store<Context>, path: &Path) -> Result<()> {
    store.data_mut().wasi.push_preopened_dir(
        Box::new(Dir::from_cap_std(CapStdDir::from_std_file(File::open(
            path,
        )?))),
        "/",
    )?;

    Ok(())
}
