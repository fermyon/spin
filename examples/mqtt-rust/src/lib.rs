use anyhow::Result;
use bytes::Bytes;
use spin_sdk::mqtt_component;
use std::str::from_utf8;

/// A simple Spin Mqtt component.
#[mqtt_component]
fn on_message(message: Bytes) -> Result<()> {
    println!("{}", from_utf8(&message)?);
    Ok(())
}
