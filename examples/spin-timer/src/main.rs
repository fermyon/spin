use anyhow::Error;
use spin_trigger::cli::FactorsTriggerCommand;

use trigger_timer::TimerTrigger;

type Command = FactorsTriggerCommand<TimerTrigger, Builder>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let t = Command::parse();
    t.run().await
}
