use super::*;
use anyhow::Result;
use spin_config::Configuration;
use std::io::Write;

const CFG_TEST: &str = r#"
name        = "spin-hello-world"
version     = "1.0.0"
description = "A simple application that returns hello and goodbye."
authors     = [ "Radu Matei <radu@fermyon.com>" ]
trigger     = { type = "http", base = "/" }

[[component]]
    source = "target/wasm32-wasi/release/hello.wasm"
    id     = "hello"
[component.trigger]
    route = "/hello"
"#;

fn read_from_temp_file(toml_text: &str) -> Result<Configuration<CoreComponent>> {
    let mut f = tempfile::NamedTempFile::new()?;
    f.write_all(toml_text.as_bytes())?;
    let config = spin_config::read_from_file(&f)?;
    drop(f);
    Ok(config)
}

#[test]
fn test_simple_config() -> Result<()> {
    let app = read_from_temp_file(CFG_TEST)?;
    let config = ExecutionContextConfiguration::new(app);

    assert_eq!(config.app.info.name, "spin-hello-world".to_string());
    Ok(())
}

#[test]
fn test_component_path() -> Result<()> {
    let test_app_origin = "dir/nested_dir";

    assert_eq!(
        complete_path(test_app_origin, "component/source.wasm"),
        PathBuf::from("dir/nested_dir/component/source.wasm")
    );

    // TODO write windows specific test
    //let abs_source_windows = "c:\\windows/nested_dir";
    let abs_source_linux = r#"/somedir/nested_dir"#;

    assert_eq!(
        complete_path(test_app_origin, abs_source_linux),
        PathBuf::from(abs_source_linux)
    );
    Ok(())
}
