use std::sync::{RwLock, Arc};

use message_io::network::SendStatus;

use crate::schema::ControllerCommand;
use crate::schema::SchedulerOperation;
use crate::schema::WorkloadEvent;

//////////////////// TODO: GENERICS Y U NO WORk /////////////////////

/////////////// SCHEDULER OPERATIONS /////////////////////

////// SENDING

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
                match ros.handler.network().send(ros.server, &body) {
                    SendStatus::Sent => (),
                    err => { anyhow::bail!("RemoteOperationSender: remote send to {} failed: {:?}", ros.addr, err); },
                }
            }
        }
        Ok(())
    }
}

pub struct RemoteOperationSender {
    addr: String,
    pub handler: message_io::node::NodeHandler<SchedulerOperation>,
    // pub listener: message_io::node::NodeListener<SchedulerOperation>,
    pub server: message_io::network::Endpoint,
}

impl RemoteOperationSender {
    pub fn new(addr: &str) -> Self {
        let (handler, _listener) = message_io::node::split();
        let (server, _) = handler.network().connect(message_io::network::Transport::FramedTcp, addr).unwrap();
        Self { addr: addr.to_owned(), handler, server }
    }
}

////// RECEIVING

#[derive(Debug)]
pub(crate) enum SchedulerOperationReceiver {
    InProcess(tokio::sync::broadcast::Receiver<SchedulerOperation>),
    Remote(RemoteOperationReceiver),
}

impl SchedulerOperationReceiver {
    pub async fn recv(&mut self) -> anyhow::Result<SchedulerOperation> {
        match self {
            Self::InProcess(c) => println!("SOR: listening in proc"),
            Self::Remote(ror) => println!("SOR: listening on {}", ror.addr),
        };
        match self {
            Self::InProcess(c) => Ok(c.recv().await?),
            Self::Remote(ror) => Ok(ror.recv().await?),
        }
    }
}

pub(crate) struct RemoteOperationReceiver {
    addr: String,
    handler: message_io::node::NodeHandler<SchedulerOperation>,
    // listener: message_io::node::NodeListener<SchedulerOperation>,
    pending: Arc<RwLock<Vec<SchedulerOperation>>>,
    node_task: message_io::node::NodeTask,
}

impl RemoteOperationReceiver {
    pub fn new(
        addr: &str,
        // handler: message_io::node::NodeHandler<SchedulerOperation>,
        // listener: message_io::node::NodeListener<SchedulerOperation>,
    ) -> Self {
        let (handler, listener) = message_io::node::split();
        handler.network().listen(message_io::network::Transport::FramedTcp, addr).unwrap();

        let pending = Arc::new(RwLock::new(vec![]));
        let pending2 = pending.clone();
        let node_task = listener.for_each_async(move |e| {
            match e {
                message_io::node::NodeEvent::Network(ne) => {
                    println!("ROR: network event");
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

        Self { addr: addr.to_owned(), handler, pending, node_task }
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

/////////////// WORKLOAD STATUS EVENTS /////////////////////

////// SENDING

#[derive(Clone)]
pub(crate) enum EventSender {
    InProcess(tokio::sync::broadcast::Sender<WorkloadEvent>),
    Remote(RemoteEventSender),
}

impl EventSender {
    pub fn send(&self, e: WorkloadEvent) -> anyhow::Result<()> {
        match self {
            Self::InProcess(c) => { c.send(e)?; },
            Self::Remote(res) => {
                let body = serde_json::to_vec(&e).unwrap();
                res.handler.network().send(res.server, &body);
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct RemoteEventSender {
    pub handler: message_io::node::NodeHandler<WorkloadEvent>,
    // pub listener: message_io::node::NodeListener<WorkloadEvent>,
    pub server: message_io::network::Endpoint,
}

impl RemoteEventSender {
    pub fn new(addr: &str) -> Self {
        let (handler, _listener) = message_io::node::split();
        let (server, _) = handler.network().connect(message_io::network::Transport::FramedTcp, addr).unwrap();
        Self { handler, server }
    }
}

////// RECEIVING

#[derive(Debug)]
pub enum WorkloadEventReceiver {
    InProcess(tokio::sync::broadcast::Receiver<WorkloadEvent>),
    Remote(RemoteEventReceiver),
}

impl WorkloadEventReceiver {
    pub async fn recv(&mut self) -> anyhow::Result<WorkloadEvent> {
        match self {
            Self::InProcess(c) => Ok(c.recv().await?),
            Self::Remote(ror) => Ok(ror.recv().await?),
        }
    }
}

pub struct RemoteEventReceiver {
    handler: message_io::node::NodeHandler<WorkloadEvent>,
    pending: Arc<RwLock<Vec<WorkloadEvent>>>,
    node_task: message_io::node::NodeTask,
}

impl RemoteEventReceiver {
    pub fn new(
        handler: message_io::node::NodeHandler<WorkloadEvent>,
        listener: message_io::node::NodeListener<WorkloadEvent>,
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

    pub async fn recv(&self) -> anyhow::Result<WorkloadEvent> {
        loop {
            match self.pop() {
                Some(o) => { return Ok(o); },
                None => tokio::time::sleep(tokio::time::Duration::from_millis(10)).await,
            }
        }
    }

    fn pop(&self) -> Option<WorkloadEvent> {
        self.pending.write().unwrap().pop()
    }
}

impl std::fmt::Debug for RemoteEventReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteEventReceiver").finish()
    }
}


/////////////// CONTROLLER COMMANDS /////////////////////

////// SENDING

#[derive(Clone)]
pub enum CommandSender {
    InProcess(tokio::sync::broadcast::Sender<ControllerCommand>),
    Remote(RemoteCommandSender),
}

impl CommandSender {
    pub fn send(&self, e: ControllerCommand) -> anyhow::Result<()> {
        match self {
            Self::InProcess(c) => { c.send(e)?; },
            Self::Remote(res) => {
                let body = serde_json::to_vec(&e).unwrap();
                res.handler.network().send(res.server, &body);
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct RemoteCommandSender {
    pub handler: message_io::node::NodeHandler<ControllerCommand>,
    // pub listener: message_io::node::NodeListener<WorkloadEvent>,
    pub server: message_io::network::Endpoint,
}

impl RemoteCommandSender {
    pub fn new(addr: &str) -> Self {
        let (handler, _listener) = message_io::node::split();
        let (server, _) = handler.network().connect(message_io::network::Transport::FramedTcp, addr).unwrap();
        Self { handler, server }
    }
}

////// RECEIVING

#[derive(Debug)]
pub(crate) enum ControllerCommandReceiver {
    InProcess(tokio::sync::broadcast::Receiver<ControllerCommand>),
    Remote(RemoteCommandReceiver),
}

impl ControllerCommandReceiver {
    pub async fn recv(&mut self) -> anyhow::Result<ControllerCommand> {
        match self {
            Self::InProcess(c) => Ok(c.recv().await?),
            Self::Remote(ror) => Ok(ror.recv().await?),
        }
    }
}

pub struct RemoteCommandReceiver {
    handler: message_io::node::NodeHandler<ControllerCommand>,
    pending: Arc<RwLock<Vec<ControllerCommand>>>,
    node_task: message_io::node::NodeTask,
}

impl RemoteCommandReceiver {
    pub fn new(
        handler: message_io::node::NodeHandler<ControllerCommand>,
        listener: message_io::node::NodeListener<ControllerCommand>,
    ) -> Self {
        let pending = Arc::new(RwLock::new(vec![]));
        let pending2 = pending.clone();
        let node_task = listener.for_each_async(move |e| {
            match e {
                message_io::node::NodeEvent::Network(ne) => {
                    match ne {
                        message_io::network::NetEvent::Message(_, body) => {
                            let msg = serde_json::from_slice(body).unwrap();
                            pending2.write().unwrap().push(msg);
                        },
                        _ => (),
                    }
                },
                _ => (),
            }
        });

        Self { handler, pending, node_task }
    }

    pub async fn recv(&self) -> anyhow::Result<ControllerCommand> {
        loop {
            match self.pop() {
                Some(o) => { return Ok(o); },
                None => tokio::time::sleep(tokio::time::Duration::from_millis(10)).await,
            }
        }
    }

    fn pop(&self) -> Option<ControllerCommand> {
        self.pending.write().unwrap().pop()
    }
}

impl std::fmt::Debug for RemoteCommandReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteCommandReceiver").finish()
    }
}

