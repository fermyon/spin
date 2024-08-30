use anyhow::Error;
use spin_trigger::cli::FactorsTriggerCommand;
use spin_cli::runtime_factors::FactorsBuilder;

use trigger_timer::TimerTrigger;

type Command = FactorsTriggerCommand<TimerTrigger, FactorsBuilder>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let t = Command::parse();
    t.run().await
}
