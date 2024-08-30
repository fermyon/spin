use anyhow::Error;
use clap::Parser;
use spin_runtime_factors::FactorsBuilder;
use spin_trigger::cli::FactorsTriggerCommand;

use trigger_timer::TimerTrigger;

type Command = FactorsTriggerCommand<TimerTrigger, FactorsBuilder>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let t = Command::parse();
    t.run().await
}
