use std::{collections::HashMap, sync::{Arc, RwLock}, path::{Path, PathBuf}};

use tempfile::TempDir;
use tokio::task::JoinHandle;

use crate::messaging::{EventSender, SchedulerOperationReceiver, RemoteOperationReceiver, RemoteEventSender};
use crate::{schema::{WorkloadId, SchedulerOperation}, store::WorkStore, WorkloadSpec, WorkloadEvent, run::run, WorkloadStatus};

#[async_trait::async_trait]
pub(crate) trait Scheduler {
    async fn notify_changed(&self, workload: &WorkloadId) -> anyhow::Result<()>;
}

struct SchedulerCore {
    store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
    running: Arc<RwLock<HashMap<WorkloadId, RunningWorkload>>>,
    event_sender: EventSender,
}

pub(crate) struct LocalScheduler {
    // store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
    // running: Arc<RwLock<HashMap<WorkloadId, RunningWorkload>>>,
    // event_sender: EventSender,
    core: SchedulerCore,
    operation_receiver: SchedulerOperationReceiver,
}

impl LocalScheduler {
    pub(crate) fn new_in_process(
        store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
        event_sender: &tokio::sync::broadcast::Sender<WorkloadEvent>,
        operation_receiver: tokio::sync::broadcast::Receiver<SchedulerOperation>
    ) -> Self {
        Self {
            core: SchedulerCore {
                store,
                running: Arc::new(RwLock::new(HashMap::new())),
                event_sender: EventSender::InProcess(event_sender.clone()),
            },
            operation_receiver: SchedulerOperationReceiver::InProcess(operation_receiver),
        }
    }

    pub(crate) fn new_rpc(
        store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
        event_sender: &tokio::sync::broadcast::Sender<WorkloadEvent>,
        operation_receiver_addr: &str,
    ) -> Self {
        let (handler, listener) = message_io::node::split();
        handler.network().listen(message_io::network::Transport::FramedTcp, operation_receiver_addr).unwrap();
        let ror = RemoteOperationReceiver::new(handler, listener);
        Self {
            core: SchedulerCore {
                store,
                running: Arc::new(RwLock::new(HashMap::new())),
                event_sender: EventSender::InProcess(event_sender.clone()),
            },
            operation_receiver: SchedulerOperationReceiver::Remote(ror),
        }
    }

    pub(crate) fn remote(
        store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
        event_addr: &str,
        operation_receiver_addr: &str,
    ) -> Self {
        let (handler, listener) = message_io::node::split();
        handler.network().listen(message_io::network::Transport::FramedTcp, operation_receiver_addr).unwrap();
        let ror = RemoteOperationReceiver::new(handler, listener);

        let event_sender = RemoteEventSender::new(event_addr);

        Self {
            core: SchedulerCore {
                store,
                running: Arc::new(RwLock::new(HashMap::new())),
                event_sender: EventSender::Remote(event_sender),
            },
            operation_receiver: SchedulerOperationReceiver::Remote(ror),
        }
    }
}

pub(crate) struct RunningWorkload {
    pub(crate) work_dir: WorkingDirectory,
    pub(crate) handle: RunHandle,
}

pub(crate) enum RunHandle {
    Fut(tokio::task::JoinHandle<()>),
}

impl LocalScheduler {
    pub fn start(self) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            self.run_event_loop().await;
        })
    }

    async fn run_event_loop(self) {
        let core = self.core;
        let mut operation_receiver = self.operation_receiver;

        loop {
            println!("SCHED: waiting to receive");
            match operation_receiver.recv().await {
                Ok(oper) => {
                    println!("SCHED: received, processing");
                    match core.process_operation(oper).await {
                        true => (),
                        false => {
                            return;
                        }
                    }
                },
                Err(e) => {
                    println!("SCHED: Oh no! {:?}", e);
                    break;
                }
            }
        }
    }
}

impl SchedulerCore {
    async fn process_operation(&self, oper: SchedulerOperation) -> bool {
        let evt = match oper {
            SchedulerOperation::WorkloadChanged(workload) =>
                self.process_workload_changed(&workload).await.err()
                    .map(|e| WorkloadEvent::UpdateFailed(workload.clone(), format!("{:#}", e))),
            SchedulerOperation::Stop => {
                for (_, h) in self.running.write().unwrap().drain() {
                    h.stop();
                }
                return false;
            }
        };

        match evt {
            None => (),
            Some(evt) => {
                match self.event_sender.send(evt) {
                    Ok(_) => (),
                    Err(_) => {
                        println!("SCHED: process_operation error, and send failed");
                    },
                }
            }
        }

        true
    }

    async fn process_workload_changed(&self, workload: &WorkloadId) -> anyhow::Result<()> {
        // TODO: look at WorkloadSpec::status
        match self.extricate(workload) {
            (Some(w), Some(c)) => {
                if w.status == WorkloadStatus::Running {
                    self.restart_workload(workload, w, c).await?
                } else {
                    self.stop_workload(workload, c)
                }
            },
            (Some(w), None) => {
                if w.status == WorkloadStatus::Running {
                    self.start_workload(workload, w).await?
                } else {
                    ()
                }
            },
            (None, Some(c)) => self.stop_workload(workload, c),
            (None, None) => (),
        }

        Ok(())
    }

    fn extricate(&self, workload: &WorkloadId) -> (Option<WorkloadSpec>, Option<RunningWorkload>) {
        let spec = self.store.read().unwrap().get_workload(workload);
        let mut running = self.running.write().unwrap();
        let current = running.remove(workload);
        (spec, current)
    }

    async fn start_workload(&self, workload: &WorkloadId, spec: WorkloadSpec) -> anyhow::Result<()> {
        // Identify the application type
        // Instantiate the relevant trigger
        // Start the relevant trigger
        let running = run(workload, spec, &self.event_sender).await?;
        // Stash the task
        self.running.write().unwrap().insert(workload.clone(), running);
        Ok(())
    }

    async fn restart_workload(&self, workload: &WorkloadId, spec: WorkloadSpec, current: RunningWorkload) -> anyhow::Result<()> {
        self.stop_workload(workload, current);
        self.start_workload(workload, spec).await
    }

    fn stop_workload(&self, workload: &WorkloadId, current: RunningWorkload) {
        current.stop();
        self.running.write().unwrap().remove(workload);
    }
}

// use std::pin::Pin;
// use futures::{Stream, StreamExt};

// fn translate_oper(msg: crate::messages::SchedulerOperation) -> SchedulerOperation {
//     todo!()
// }

// #[async_trait::async_trait]
// impl crate::messages::scheduler_server::Scheduler for RemoteScheduler {
//     type SchedulerStream = Pin<Box<dyn Stream<Item = Result<crate::messages::WorkloadEvent, tonic::Status>> + Send + 'static>>;
//     async fn scheduler(&self, request: tonic::Request<tonic::Streaming<crate::messages::SchedulerOperation> >) -> Result<tonic::Response<Self::SchedulerStream>, tonic::Status> {  // Pin<Box<(dyn futures::Future<Output = Result<tonic::Response<<Self as scheduler_server::Scheduler>::SchedulerStream>, Status>> + std::marker::Send + 'async_trait)>> { todo!() }`: `fn scheduler(&'life0 self, _: tonic::Request<Streaming<messages::SchedulerOperation>>) -> Pin<Box<(dyn futures::Future<Output = Result<tonic::Response<<Self as scheduler_server::Scheduler>::SchedulerStream>, Status>> + std::marker::Send + 'async_trait)>>
//         let mut stream = request.into_inner();

//         let output = async_stream::try_stream! {
//             while let Some(oper) = stream.next().await {
//                 let oper = oper?;
//                 let message = oper.message.clone().unwrap();

//                 let roper = translate_oper(message);
//                 let should_continue = process_operation(roper).await;
//                 if !should_continue {
//                     break;
//                 }

//                 for note in location_notes {
//                     yield note.clone();
//                 }
//             }
//         };

//         Ok(Response::new(Box::pin(output) as Self::RouteChatStream))
//     }
// }

impl RunningWorkload {
    fn stop(self) {
        match self.handle {
            RunHandle::Fut(f) => f.abort(),
        }
        drop(self.work_dir);
    }
}

pub(crate) enum WorkingDirectory {
    Given(PathBuf),
    Temporary(TempDir),
}

impl WorkingDirectory {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Given(p) => p,
            Self::Temporary(t) => t.path(),
        }
    }
}
