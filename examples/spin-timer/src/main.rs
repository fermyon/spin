use anyhow::Error;
use clap::Parser;
use spin_trigger::cli::TriggerExecutorCommand;
use trigger_timer::TimerTrigger;

type Command = TriggerExecutorCommand<TimerTrigger>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let t = Command::parse();
    t.run().await
}
