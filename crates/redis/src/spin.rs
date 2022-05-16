use crate::{spin_redis::SpinRedis, ExecutionContext, RedisExecutor, RuntimeContext};
use anyhow::Result;
use async_trait::async_trait;
use spin_engine::io::{ModuleIoRedirects, ModuleIoRedirectsTypes};
use tokio::task::spawn_blocking;
use wasmtime::{Instance, Store};

#[derive(Clone)]
pub struct SpinRedisExecutor;

#[async_trait]
impl RedisExecutor for SpinRedisExecutor {
    async fn execute(
        &self,
        engine: &ExecutionContext,
        component: &str,
        channel: &str,
        payload: &[u8],
        follow: bool,
    ) -> Result<()> {
        log::trace!(
            "Executing request using the Spin executor for component {}",
            component
        );

        let mior: Option<ModuleIoRedirects>;

        if follow {
            mior = match engine.config.module_io_redirects.clone() {
                ModuleIoRedirectsTypes::Default => Some(ModuleIoRedirects::new()),
                ModuleIoRedirectsTypes::FromFiles(clp) => Some(ModuleIoRedirects::new_from_files(
                    clp.stdin_pipe.0,
                    clp.stdout_pipe.0,
                    clp.stderr_pipe.0,
                )),
            };
        } else {
            mior = None;
        }

        let (store, instance) = engine.prepare_component(
            component,
            None,
            match mior.clone() {
                Some(mr) => Some(mr.pipes),
                None => None,
            },
            None,
            None,
        )?;

        let result = match Self::execute_impl(store, instance, channel, payload.to_vec()).await {
            Ok(()) => {
                log::trace!("Request finished OK");
                Ok(())
            }
            Err(e) => {
                log::trace!("Request finished with error {}", e);
                Err(e)
            }
        };

        let log_result = engine.save_output_to_logs(
            mior.unwrap().read_handles.read(),
            component,
            true,
            true,
        );

        result.and(log_result)
    }
}

impl SpinRedisExecutor {
    pub async fn execute_impl(
        mut store: Store<RuntimeContext>,
        instance: Instance,
        _channel: &str,
        payload: Vec<u8>,
    ) -> Result<()> {
        let engine = SpinRedis::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;

        let _res = spawn_blocking(move || -> Result<crate::spin_redis::Error> {
            match engine.handle_redis_message(&mut store, &payload) {
                Ok(_) => Ok(crate::spin_redis::Error::Success),
                Err(_) => Ok(crate::spin_redis::Error::Error),
            }
        })
        .await??;

        Ok(())
    }
}
