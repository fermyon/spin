use crate::{wit::wasi::poll::poll2 as poll, WasiCloud};
use anyhow::{anyhow, Result};
use spin_core::async_trait;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Wake,
};
use tokio::sync::Notify;

#[derive(Clone)]
pub struct Pollable(pub Arc<AtomicBool>);

pub struct PollWaker {
    pub pollable: Pollable,
    pub notify: Arc<Notify>,
}

impl Wake for PollWaker {
    fn wake(self: Arc<Self>) {
        self.pollable.0.store(true, Ordering::SeqCst);
        self.notify.notify_one();
    }
}

#[async_trait]
impl poll::Host for WasiCloud {
    async fn drop_pollable(&mut self, this: poll::Pollable) -> Result<()> {
        self.pollables.remove(this);
        Ok(())
    }

    async fn poll_oneoff(&mut self, pollables: Vec<poll::Pollable>) -> Result<Vec<bool>> {
        let pollables = pollables
            .iter()
            .map(|handle| {
                self.pollables
                    .get(*handle)
                    .ok_or_else(|| anyhow!("unknown handle: {handle}"))
            })
            .collect::<Result<Vec<_>>>()?;

        loop {
            let mut ready = false;
            let result = pollables
                .iter()
                .map(|pollable| {
                    if pollable.0.load(Ordering::SeqCst) {
                        ready = true;
                        true
                    } else {
                        false
                    }
                })
                .collect();

            if ready {
                break Ok(result);
            } else {
                self.notify.notified().await;
            }
        }
    }
}
