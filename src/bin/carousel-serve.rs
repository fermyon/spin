use anyhow::Error;

const CMD_ADDR: &str = "127.0.0.1:4646";

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(atty::is(atty::Stream::Stderr))
        .init();

    let controller = spin_controller::StandaloneController::new(CMD_ADDR, "127.0.0.1:4647", "127.0.0.1:3636");

    let controller_jh = controller.start();

    println!("Controller running, accepting commands on {}", CMD_ADDR);

    controller_jh.await?;

    Ok(())
}
