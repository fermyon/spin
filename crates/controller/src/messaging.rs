use std::sync::{RwLock, Arc};

use crate::schema::SchedulerOperation;
use crate::schema::WorkloadEvent;

pub enum SchedulerOperationSender {
    InProcess(tokio::sync::broadcast::Sender<SchedulerOperation>),
    Remote(RemoteOperationSender),
}

impl SchedulerOperationSender {
    pub fn send(&self, oper: SchedulerOperation) -> anyhow::Result<()> {
        match self {
            Self::InProcess(c) => { c.send(oper)?; },
            Self::Remote(ros) => {
                let body = serde_json::to_vec(&oper).unwrap();
                ros.handler.network().send(ros.server, &body);
                // ros.handler.signals().send(oper);
            }
        }
        Ok(())
    }
}

pub struct RemoteOperationSender {
    pub handler: message_io::node::NodeHandler<SchedulerOperation>,
    pub listener: message_io::node::NodeListener<SchedulerOperation>,
    pub server: message_io::network::Endpoint,
}

#[derive(Clone, Debug)]
pub(crate) enum EventSender {
    InProcess(tokio::sync::broadcast::Sender<WorkloadEvent>),
}

#[derive(Debug)]
pub(crate) enum OperationReceiver {
    InProcess(tokio::sync::broadcast::Receiver<SchedulerOperation>),
    Remote(RemoteOperationReceiver),
}

impl EventSender {
    pub fn send(&self, e: WorkloadEvent) -> anyhow::Result<()> {
        match self {
            Self::InProcess(c) => { c.send(e)?; },
        }
        Ok(())
    }
}

impl OperationReceiver {
    pub async fn recv(&mut self) -> anyhow::Result<SchedulerOperation> {
        match self {
            Self::InProcess(c) => Ok(c.recv().await?),
            Self::Remote(ror) => Ok(ror.recv().await?),
        }
    }
}

pub(crate) struct RemoteOperationReceiver {
    handler: message_io::node::NodeHandler<SchedulerOperation>,
    // listener: message_io::node::NodeListener<SchedulerOperation>,
    pending: Arc<RwLock<Vec<SchedulerOperation>>>,
    node_task: message_io::node::NodeTask,
}

impl RemoteOperationReceiver {
    pub fn new(
        handler: message_io::node::NodeHandler<SchedulerOperation>,
        listener: message_io::node::NodeListener<SchedulerOperation>,
    ) -> Self {
        let pending = Arc::new(RwLock::new(vec![]));
        let pending2 = pending.clone();
        let node_task = listener.for_each_async(move |e| {
            match e {
                message_io::node::NodeEvent::Network(ne) => {
                    match ne {
                        message_io::network::NetEvent::Message(_, body) => {
                            let oper = serde_json::from_slice(body).unwrap();
                            pending2.write().unwrap().push(oper);
                        },
                        _ => (),
                    }
                },
                _ => (),
            }
        });

        Self { handler, pending, node_task }
    }

    pub async fn recv(&self) -> anyhow::Result<SchedulerOperation> {
        loop {
            match self.pop() {
                Some(o) => { return Ok(o); },
                None => tokio::time::sleep(tokio::time::Duration::from_millis(10)).await,
            }
        }
    }

    fn pop(&self) -> Option<SchedulerOperation> {
        self.pending.write().unwrap().pop()
    }
}

impl std::fmt::Debug for RemoteOperationReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteOperationReceiver").finish()
    }
}
