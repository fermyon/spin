use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str;
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    process::{self, Command, Output},
    time::Duration,
};

use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tokio::{net::TcpStream, time::sleep};

/// Run the command and returns the output
pub fn run<S: AsRef<str>>(
    args: &[S],
    dir: Option<&Path>,
    envs: Option<HashMap<&str, &str>>,
) -> Result<Output> {
    let mut cmd = Command::new(get_os_process());
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());

    if let Some(dir) = dir {
        cmd.current_dir(dir);
    };

    cmd.arg("-c");
    cmd.arg(args.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" "));
    if let Some(envs) = envs {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }

    println!("Running command: {cmd:?}");
    Ok(cmd.output()?)
}

/// Asserts that the output from a `Command` was a success (i.e., exist status was 0)
pub fn assert_success(output: &Output) {
    let code = output
        .status
        .code()
        .expect("process unexpectedly terminated by a signal");
    if code != 0 {
        let stdout = std::str::from_utf8(&output.stdout).unwrap_or("<STDOUT is not UTF8>");
        let stderr = std::str::from_utf8(&output.stderr).unwrap_or("<STDERR is not UTF8>");
        panic!("exited with code {code}.\n\nstdout:\n{stdout}\n\nstderr:\n{stderr}\n",);
    }
}

fn get_os_process() -> String {
    if cfg!(target_os = "windows") {
        String::from("powershell.exe")
    } else {
        String::from("/bin/bash")
    }
}

/// gets a free random port
pub fn get_random_port() -> Result<u16> {
    Ok(
        TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?
            .local_addr()?
            .port(),
    )
}

/// wait for tcp to work on a given port
///
/// Return `Ok(true)` if the tcp port can be connected and
/// `Ok(false)` if the process exited early
pub async fn wait_tcp(
    url: &str,
    process: &mut tokio::process::Child,
    target: &str,
) -> Result<bool> {
    let mut wait_count = 0;
    while wait_count < 240 {
        if let Ok(Some(_)) = process.try_wait() {
            return Ok(false);
        }
        if TcpStream::connect(&url).await.is_ok() {
            return Ok(true);
        }

        wait_count += 1;
        sleep(Duration::from_secs(1)).await;
    }

    Err(anyhow!(
        "Ran out of retries waiting for {} to start on URL {}",
        target,
        url
    ))
}

/// run the process in background and returns a tokio::process::Child
/// the caller can then call utils::get_output(&mut child).await
/// to get logs from the stdout
pub fn run_async<S: AsRef<str>>(
    args: &[S],
    dir: Option<&Path>,
    envs: Option<HashMap<&str, &str>>,
) -> tokio::process::Child {
    let mut cmd = TokioCommand::new(get_os_process());
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());

    if let Some(dir) = dir {
        cmd.current_dir(dir);
    };

    cmd.arg("-c");
    cmd.arg(args.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" "));
    if let Some(envs) = envs {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }

    cmd.spawn().expect("failed to spawn command")
}

pub fn testcases_base_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/testcases")
}

pub async fn get_output_stream(
    reader: Option<Pin<Box<dyn AsyncBufRead>>>,
    max_wait: Duration,
) -> Result<Vec<String>> {
    let Some(mut reader) = reader else {
        let output: Result<Vec<String>, anyhow::Error> = Ok(vec![]);
        return output;
    };

    let mut output: Vec<String> = vec![];

    loop {
        let mut line = String::new();
        let nextline = reader.read_line(&mut line);
        match timeout(max_wait, nextline).await {
            Ok(Ok(n)) if n > 0 => {
                line.pop(); // pop the newline off
                output.push(line)
            }
            _ => break,
        }
    }

    Ok(output)
}

pub async fn get_output(reader: Option<Pin<Box<dyn AsyncBufRead>>>) -> Result<String> {
    let stream = get_output_stream(reader, Duration::from_secs(1)).await?;
    Ok(stream.join("\n"))
}
