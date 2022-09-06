use anyhow::Result;
use clap::Parser;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};
use wasmtime::{Config, Engine, Module};

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Options {
    /// Name of Wasm file to test (or stdin if not specified)
    #[clap(short, long)]
    pub input: Option<PathBuf>,

    /// Name of JSON file to write report to (or stdout if not specified)
    #[clap(short, long)]
    pub output: Option<PathBuf>,

    /// Name of TOML configuration file to use
    #[clap(short, long)]
    pub config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let options = &Options::parse();

    let engine = &Engine::new(&Config::new())?;

    let module = &if let Some(input) = &options.input {
        Module::from_file(engine, input)
    } else {
        Module::new(engine, &{
            let mut buffer = Vec::new();
            io::stdin().read_to_end(&mut buffer)?;
            buffer
        })
    }?;

    let config = if let Some(config) = &options.config {
        toml::from_str(&fs::read_to_string(config)?)?
    } else {
        spin_abi_conformance::Config::default()
    };

    let report = &spin_abi_conformance::test(module, config)?;

    let writer = if let Some(output) = &options.output {
        Box::new(File::create(output)?) as Box<dyn Write>
    } else {
        Box::new(io::stdout().lock())
    };

    serde_json::to_writer_pretty(writer, report)?;

    Ok(())
}
