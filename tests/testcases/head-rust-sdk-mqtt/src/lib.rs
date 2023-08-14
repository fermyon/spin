use spin_sdk::mqtt_component;

#[mqtt_component]
fn on_message(message: bytes::Bytes) -> anyhow::Result<()> {
    println!(
        "Got message: '{}'",
        std::str::from_utf8(&*message).unwrap_or("<MESSAGE NOT UTF8>")
    );
    Ok(())
}
