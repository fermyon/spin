use spin_sdk::redis_component;
use std::str::from_utf8;

/// A simple Spin Redis component.
#[redis_component]
fn on_message(message: Vec<u8>) -> anyhow::Result<()> {
    println!("{}", from_utf8(&message)?);
    Ok(())
}
