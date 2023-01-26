use anyhow::Result;
use std::path::PathBuf;
use std::str;
use std::{
    collections::HashMap,
    ffi::OsStr,
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
    process::{self, Command, Output},
    time::Duration,
};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tokio::{net::TcpStream, time::sleep};

/// run the command and returns the output
pub fn run<S: Into<String> + AsRef<OsStr>>(
    args: Vec<S>,
    dir: Option<S>,
    envs: Option<HashMap<&str, &str>>,
) -> Result<Output> {
    let mut cmd = Command::new(get_os_process());
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());

    if let Some(dir) = dir {
        cmd.current_dir(dir.into());
    };

    cmd.arg("-c");
    cmd.arg(
        args.into_iter()
            .map(Into::into)
            .collect::<Vec<String>>()
            .join(" "),
    );
    if let Some(envs) = envs {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }

    let output = cmd.output()?;
    let code = output.status.code().expect("should have status code");
    if code != 0 {
        println!("{:#?}", std::str::from_utf8(&output.stderr)?);
        println!("{:#?}", std::str::from_utf8(&output.stdout)?);
        panic!("command `{:?}` exited with code {}", cmd, code);
    }

    Ok(output)
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
pub async fn wait_tcp(url: &str, process: &mut tokio::process::Child, target: &str) -> Result<()> {
    let mut wait_count = 0;
    loop {
        if wait_count >= 240 {
            panic!(
                "Ran out of retries waiting for {} to start on URL {}",
                target, url
            );
        }

        if let Ok(Some(_)) = process.try_wait() {
            panic!(
                "Process exited before starting to serve {} to start on URL {}",
                target, url
            );
        }

        match TcpStream::connect(&url).await {
            Ok(_) => break,
            Err(e) => {
                println!("connect {} error {}, retry {}", &url, e, wait_count);
                wait_count += 1;
                sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}

/// run the process in background and returns a tokio::process::Child
/// the caller can then call utils::get_output(&mut child).await
/// to get logs from the stdout
pub fn run_async<S: Into<String> + AsRef<OsStr>>(
    args: Vec<S>,
    dir: Option<S>,
    envs: Option<HashMap<&str, &str>>,
) -> tokio::process::Child {
    let mut cmd = TokioCommand::new(get_os_process());
    cmd.stdout(process::Stdio::piped());
    cmd.stderr(process::Stdio::piped());

    if let Some(dir) = dir {
        cmd.current_dir(dir.into());
    };

    cmd.arg("-c");
    cmd.arg(
        args.into_iter()
            .map(Into::into)
            .collect::<Vec<String>>()
            .join(" "),
    );
    if let Some(envs) = envs {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }

    return cmd.spawn().expect("failed to spawn command");
}

/// gets the stdout of the tokio::process::Child spawned in the background
///
/// it blocks for the first line of logs and then
/// reads all the remaining lines of logs from stdout
/// if no new logs are received in 5 seconds, it returns
/// the logs collected so far
pub async fn get_output(child: &mut tokio::process::Child) -> Result<Vec<String>> {
    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let mut reader = BufReader::new(stdout).lines();

    //get firstline in a blocking way to ensure we account for `spin up` delay
    let firstline_future = reader.next_line();
    let firstline = timeout(Duration::from_secs(20), firstline_future)
        .await?
        .unwrap()
        .unwrap();
    let mut output = vec![firstline];

    loop {
        let nextline = reader.next_line();
        match timeout(Duration::from_secs(5), nextline).await {
            Err(_) => break,
            Ok(result) => match result {
                Err(_) => break,
                Ok(line) => output.push(line.unwrap()),
            },
        }
    }

    Ok(output)
}

pub fn testcases_base_dir() -> String {
    let basedir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "../../tests/testcases"]
        .iter()
        .collect();

    basedir.to_str().unwrap().to_string()
}
