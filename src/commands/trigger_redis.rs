use anyhow::Result;
use clap::Parser;
use spin_redis_engine::RedisTrigger;
use spin_trigger::{run_trigger, ExecutionOptions};

use super::trigger::TriggerCommonOpts;

/// Run the build command for each component.
#[derive(Parser, Debug)]
#[clap(about = "Run the Redis trigger executor")]
pub struct TriggerRedisCommand {
    #[clap(flatten)]
    pub opts: TriggerCommonOpts,
}

impl TriggerRedisCommand {
    pub async fn run(&self) -> Result<()> {
        let app = self.opts.app_from_env().await?;

        run_trigger(
            app,
            ExecutionOptions::<RedisTrigger>::new(
                self.opts.kv_dir.clone(),
                self.opts.log.clone(),
                self.opts.follow_components(),
                (),
            ),
            Some(self.opts.wasmtime_config()?),
        )
        .await
    }
}
