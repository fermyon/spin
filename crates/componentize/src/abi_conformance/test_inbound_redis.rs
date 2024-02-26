use super::{
    redis_types::{Error, Payload},
    Context, TestConfig,
};
use anyhow::anyhow;
use wasmtime::{component::InstancePre, Engine};

pub(crate) async fn test(
    engine: &Engine,
    test_config: TestConfig,
    pre: &InstancePre<Context>,
) -> Result<(), String> {
    super::run(async {
        let mut store = super::create_store(engine, test_config);
        let instance = pre.instantiate_async(&mut store).await?;

        let func = instance
            .exports(&mut store)
            .instance("fermyon:spin/inbound-redis")
            .ok_or_else(|| anyhow!("no inbound-redis instance found"))?
            .typed_func::<(Payload,), (Result<(), Error>,)>("handle-message")?;

        match func
            .call_async(store, (b"Hello, SpinRedis!".to_vec(),))
            .await?
        {
            (Ok(()) | Err(Error::Success),) => Ok(()),
            (Err(e),) => Err(e.into()),
        }
    })
    .await
}
