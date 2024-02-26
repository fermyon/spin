use std::{
    env,
    fs::{self, File},
    io, iter, str,
    time::SystemTime,
};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Config {
        key: String,
    },
    Http {
        url: String,
    },
    RedisPublish {
        address: String,
        key: String,
        value: String,
    },
    RedisSet {
        address: String,
        key: String,
        value: String,
    },
    RedisGet {
        address: String,
        key: String,
    },
    RedisIncr {
        address: String,
        key: String,
    },
    RedisDel {
        address: String,
        keys: Vec<String>,
    },
    RedisSadd {
        address: String,
        key: String,
        params: Vec<String>,
    },
    RedisSrem {
        address: String,
        key: String,
        params: Vec<String>,
    },
    RedisSmembers {
        address: String,
        key: String,
    },
    RedisExecute {
        address: String,
        command: String,
        params: Vec<String>,
    },
    PostgresExecute {
        address: String,
        statement: String,
        params: Vec<String>,
    },
    PostgresQuery {
        address: String,
        statement: String,
        params: Vec<String>,
    },
    MysqlExecute {
        address: String,
        statement: String,
        params: Vec<String>,
    },
    MysqlQuery {
        address: String,
        statement: String,
        params: Vec<String>,
    },
    KeyValueOpen {
        name: String,
    },
    KeyValueGet {
        store: u32,
        key: String,
    },
    KeyValueSet {
        store: u32,
        key: String,
        value: String,
    },
    KeyValueDelete {
        store: u32,
        key: String,
    },
    KeyValueExists {
        store: u32,
        key: String,
    },
    KeyValueGetKeys {
        store: u32,
    },
    KeyValueClose {
        store: u32,
    },
    LlmInfer {
        model: String,
        prompt: String,
    },
    WasiEnv {
        key: String,
    },
    WasiEpoch,
    WasiRandom,
    WasiStdio,
    WasiRead {
        file_name: String,
    },
    WasiReaddir {
        dir_name: String,
    },
    WasiStat {
        file_name: String,
    },
}

impl Command {
    pub fn extract(body: Option<Vec<u8>>) -> anyhow::Result<Command> {
        let body = body.ok_or_else(|| anyhow::anyhow!("empty request body"))?;
        let command = iter::once("<wasm module>")
            .chain(str::from_utf8(&body)?.split("%20"))
            .collect::<Vec<_>>();
        Ok(Cli::try_parse_from(command)?.command)
    }

    pub fn read_dir(dir_name: String) -> anyhow::Result<()> {
        let mut comma = false;
        Ok(for entry in fs::read_dir(dir_name)? {
            if comma {
                print!(",");
            } else {
                comma = true;
            }

            print!(
                "{}",
                entry?
                    .file_name()
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("non-UTF-8 file name"))?
            );
        })
    }

    pub fn read(file_name: String) -> Result<(), anyhow::Error> {
        io::copy(&mut File::open(file_name)?, &mut io::stdout().lock())?;
        Ok(())
    }

    pub fn stdio() -> Result<(), anyhow::Error> {
        io::copy(&mut io::stdin().lock(), &mut io::stdout().lock())?;
        Ok(())
    }

    pub fn stat(file_name: String) -> Result<(), anyhow::Error> {
        let metadata = fs::metadata(file_name)?;
        print!(
            "length:{},modified:{}",
            metadata.len(),
            metadata
                .modified()?
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_millis()
        );
        Ok(())
    }

    pub fn random() -> Result<(), anyhow::Error> {
        let mut buffer = [0u8; 8];
        getrandom::getrandom(&mut buffer).map_err(|_| anyhow::anyhow!("getrandom error"))?;
        Ok(())
    }

    pub fn env(key: String) -> anyhow::Result<()> {
        print!("{}", env::var(key)?);
        Ok(())
    }

    pub fn epoch() -> anyhow::Result<()> {
        print!(
            "{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_millis()
        );
        Ok(())
    }
}

pub fn split_param(param: &str) -> Result<(&str, &str), anyhow::Error> {
    let (type_, value) = param
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("expected ':' in {param}"))?;
    Ok((type_, value))
}
