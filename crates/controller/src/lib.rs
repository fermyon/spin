use std::sync::{RwLock, Arc};

use messaging::{SchedulerOperationSender, RemoteOperationSender, SchedulerOperationReceiver, EventSender, ControllerCommandReceiver, RemoteCommandReceiver, RemoteEventSender};
pub use messaging::{CommandSender, RemoteCommandSender, RemoteEventReceiver, WorkloadEventReceiver};
use scheduler::{LocalScheduler};
use schema::{SchedulerOperation};
pub use schema::{ControllerCommand, WorkloadEvent, WorkloadId, WorkloadManifest, WorkloadOpts, WorkloadSpec, WorkloadStatus};
use store::{WorkStore, InMemoryWorkStore};
use tokio::task::JoinHandle;

mod messaging;
mod run;
pub(crate) mod scheduler;
pub(crate) mod schema;
pub(crate) mod store;

pub struct Controller {
    core: ControllerCore,
    scheduler_task: tokio::task::JoinHandle<()>,
    client_cmd_receiver: ControllerCommandReceiver,
    client_evt_notifier: EventSender,
    sched_evt_receiver: WorkloadEventReceiver,
}

pub struct StandaloneController {
    core: ControllerCore,
    scheduler_task: tokio::task::JoinHandle<()>,
    client_cmd_receiver: ControllerCommandReceiver,
    client_evt_notifiers: Arc<RwLock<Vec<EventSender>>>,
    sched_evt_receiver: WorkloadEventReceiver,
}

impl StandaloneController {
    pub fn new(ctrl_cmd_addr: &str, ctrl_evt_addr: &str, sched_addr: &str) -> Self {
        let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));

        let (ctrl_evt_handler, ctrl_evt_listener) = message_io::node::split();
        ctrl_evt_handler.network().listen(message_io::network::Transport::FramedTcp, ctrl_evt_addr).unwrap();
        let sched_evt_rx = RemoteEventReceiver::new(ctrl_evt_handler, ctrl_evt_listener);

        let scheduler = LocalScheduler::remote(store.clone(), ctrl_evt_addr, sched_addr);
        let scheduler_task = scheduler.start();

        // TODO: this should probably not attempt to connect in its constructor
        let sched_oper_tx = RemoteOperationSender::new(sched_addr);
        let scheduler_notifier = SchedulerOperationSender::Remote(sched_oper_tx);

        let (client_cmd_handler, client_cmd_listener) = message_io::node::split();
        client_cmd_handler.network().listen(message_io::network::Transport::FramedTcp, ctrl_cmd_addr).unwrap();
        let client_cmd_rx = RemoteCommandReceiver::new(client_cmd_handler, client_cmd_listener);

        let core = ControllerCore {
            store,
            scheduler_notifier,
        };
        let client_cmd_receiver = ControllerCommandReceiver::Remote(client_cmd_rx);
        let sched_evt_receiver = WorkloadEventReceiver::Remote(sched_evt_rx);

        let controller = Self {
            core,
            scheduler_task,
            client_cmd_receiver,
            client_evt_notifiers: Arc::new(RwLock::new(vec![])),
            sched_evt_receiver,
        };

        controller
    }

    pub fn start(self) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            self.run_event_loop().await;
        })
    }

    async fn run_event_loop(self) {
        let mut core = self.core;
        let mut receiver = self.client_cmd_receiver;
        let mut evt_receiver = self.sched_evt_receiver;

        loop {
            println!("CTRL: waiting to receive");
            tokio::select! {
                cmd = receiver.recv() => {
                    match cmd {
                        Ok(ControllerCommand::Connect(addr)) => {
                            println!("CTRL: connection received from {}", addr);
                            let tx = RemoteEventSender::new(&addr);
                            self.client_evt_notifiers.write().unwrap().push(EventSender::Remote(tx));
                        },
                        Ok(msg) => {
                            println!("CTRL: received, processing");
                            match core.process_message(msg) {
                                Ok(true) => (),
                                Ok(false) => { return; }
                                Err(e) => { println!("CTRL: processing failed {:#}", e); return; }
                            }
                        },
                        Err(e) => {
                            println!("CTRL CMD: Oh no! {:?}", e);
                            break;
                        }
                    }
                },
                evt = evt_receiver.recv() => {
                    match evt {
                        Ok(e) => {
                            let notifs = self.client_evt_notifiers.read().unwrap();
                            for notif in notifs.iter() {
                                if let Err(err) = notif.send(e.clone()) {
                                    println!("CTRL EVT: send err {:?}", err);
                                }
                            }
                        },
                        Err(e) => {
                            println!("CTRL EVT: Oh no! {:?}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl Controller {
    pub fn in_memory() -> (Self, CommandSender, WorkloadEventReceiver) {
        let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));

        let (sched_evt_tx, sched_evt_rx) = tokio::sync::broadcast::channel(1000);
        let (sched_oper_tx, sched_oper_rx) = tokio::sync::broadcast::channel(1000);
        let (client_cmd_tx, client_cmd_rx) = tokio::sync::broadcast::channel(1000);
        let (client_evt_tx, client_evt_rx) = tokio::sync::broadcast::channel(1000);

        let scheduler = LocalScheduler::new_in_process(store.clone(), &sched_evt_tx, sched_oper_rx);
        let scheduler_task = scheduler.start();
        let scheduler_notifier = SchedulerOperationSender::InProcess(sched_oper_tx);

        let core = ControllerCore {
            store,
            scheduler_notifier,
        };
        let client_cmd_receiver = ControllerCommandReceiver::InProcess(client_cmd_rx);
        let client_evt_notifier = EventSender::InProcess(client_evt_tx);
        let sched_evt_receiver = WorkloadEventReceiver::InProcess(sched_evt_rx);

        let controller = Self {
            core,
            scheduler_task,
            client_cmd_receiver,
            client_evt_notifier,
            sched_evt_receiver,
        };

        let client_cmd_sender = CommandSender::InProcess(client_cmd_tx);
        let client_evt_receiver = WorkloadEventReceiver::InProcess(client_evt_rx);
        (controller, client_cmd_sender, client_evt_receiver)
    }

    pub fn in_memory_sched_rpc(sched_addr: &str) -> (Self, CommandSender, WorkloadEventReceiver) {
        let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));

        let (sched_evt_tx, sched_evt_rx) = tokio::sync::broadcast::channel(1000);
        let (client_cmd_tx, client_cmd_rx) = tokio::sync::broadcast::channel(1000);
        let (client_evt_tx, client_evt_rx) = tokio::sync::broadcast::channel(1000);

        let scheduler = LocalScheduler::new_rpc(store.clone(), &sched_evt_tx, sched_addr);
        let scheduler_task = scheduler.start();

        // TODO: this should probably not attempt to connect in its constructor
        let sched_oper_tx = RemoteOperationSender::new(sched_addr);
        let scheduler_notifier = SchedulerOperationSender::Remote(sched_oper_tx);

        let core = ControllerCore {
            store,
            scheduler_notifier,
        };
        let client_cmd_receiver = ControllerCommandReceiver::InProcess(client_cmd_rx);
        let client_evt_notifier = EventSender::InProcess(client_evt_tx);
        let sched_evt_receiver = WorkloadEventReceiver::InProcess(sched_evt_rx);

        let controller = Self {
            core,
            scheduler_task,
            client_cmd_receiver,
            client_evt_notifier,
            sched_evt_receiver,
        };

        let client_cmd_sender = CommandSender::InProcess(client_cmd_tx);
        let client_evt_receiver = WorkloadEventReceiver::InProcess(client_evt_rx);
        (controller, client_cmd_sender, client_evt_receiver)
    }

    pub fn remote(client_addr: &str, ctrl_cmd_addr: &str, ctrl_evt_addr: &str, sched_addr: &str) -> Self {
        let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));

        let scheduler = LocalScheduler::remote(store.clone(), client_addr, sched_addr);
        let scheduler_task = scheduler.start();

        // TODO: this should probably not attempt to connect in its constructor
        let sched_oper_tx = RemoteOperationSender::new(sched_addr);
        let scheduler_notifier = SchedulerOperationSender::Remote(sched_oper_tx);

        let (ctrl_evt_handler, ctrl_evt_listener) = message_io::node::split();
        ctrl_evt_handler.network().listen(message_io::network::Transport::FramedTcp, ctrl_evt_addr).unwrap();
        let sched_evt_rx = RemoteEventReceiver::new(ctrl_evt_handler, ctrl_evt_listener);

        let (client_cmd_handler, client_cmd_listener) = message_io::node::split();
        client_cmd_handler.network().listen(message_io::network::Transport::FramedTcp, ctrl_cmd_addr).unwrap();
        let client_cmd_rx = RemoteCommandReceiver::new(client_cmd_handler, client_cmd_listener);

        let client_evt_tx = RemoteEventSender::new(client_addr);

        let core = ControllerCore {
            store,
            scheduler_notifier,
        };
        let client_cmd_receiver = ControllerCommandReceiver::Remote(client_cmd_rx);
        let client_evt_notifier = EventSender::Remote(client_evt_tx);
        let sched_evt_receiver = WorkloadEventReceiver::Remote(sched_evt_rx);

        let controller = Self {
            core,
            scheduler_task,
            client_cmd_receiver,
            client_evt_notifier,
            sched_evt_receiver,
        };

        controller
    }

    pub fn start(self) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            self.run_event_loop().await;
        })
    }

    async fn run_event_loop(self) {
        let mut core = self.core;
        let mut receiver = self.client_cmd_receiver;

        loop {
            println!("CTRL: waiting to receive");
            match receiver.recv().await {
                Ok(msg) => {
                    println!("CTRL: received, processing");
                    match core.process_message(msg) {
                        Ok(true) => (),
                        Ok(false) => { return; }
                        Err(e) => { println!("CTRL: processing failed {:#}", e); return; }
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


struct ControllerCore {
    store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
    scheduler_notifier: SchedulerOperationSender, // tokio::sync::broadcast::Sender<SchedulerOperation>,
}

impl ControllerCore {
    pub fn process_message(&mut self, msg: ControllerCommand) -> anyhow::Result<bool> {
        match msg {
            ControllerCommand::SetWorkload(workload, spec) =>
                self.set_workload(&workload, spec)?,
            ControllerCommand::RemoveWorkload(workload) =>
                self.remove_workload(&workload)?,
            ControllerCommand::Shutdown => {
                self.shutdown()?;
                return Ok(false);
            },
            ControllerCommand::Connect(_) => (),  // Handled at a higher level. Yes, messy
        };

        println!("CTRL: message sent to scheduler");
        Ok(true)
    }

    pub fn set_workload(&mut self, workload: &WorkloadId, spec: WorkloadSpec) -> anyhow::Result<()> {
        self.store.write().unwrap().set_workload(workload, spec);
        let oper = SchedulerOperation::WorkloadChanged(workload.clone());
        self.scheduler_notifier.send(oper)?;
        Ok(())
    }

    pub fn remove_workload(&mut self, workload: &WorkloadId) -> anyhow::Result<()> {
        self.store.write().unwrap().remove_workload(workload);
        let oper = SchedulerOperation::WorkloadChanged(workload.clone());
        self.scheduler_notifier.send(oper)?;
        Ok(())
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.scheduler_notifier.send(SchedulerOperation::Stop)?;
        Ok(())
    }
}

// impl RpcControl {
//     pub async fn new() -> anyhow::Result<Self> {
//         // let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
//         // let store = Arc::new(RwLock::new(box_store));
//         // let (evt_tx, evt_rx) = tokio::sync::broadcast::channel(1000);
//         // let (oper_tx, oper_rx) = std::sync::mpsc::channel();
//         // let scheduler = RemoteScheduler::new(store.clone(), &evt_tx);
//         // let jh = scheduler.start_server("[::1]:10000");
//         // println!("CTRL: CONNECTING");
//         // let scheduler_client = crate::messages::scheduler_client::SchedulerClient::connect("http://[::1]:10000").await?;
//         // println!("CTRL: CONNECTED");
//         // let jh2 = tokio::task::spawn(Self::run_server_session(
//         //     scheduler_client.clone(),
//         //     oper_rx,
//         //     evt_tx.clone(),
//         // ));
//         Ok(Self {
//             // scheduler: jh,
//             // store,
//             // event_sender: evt_tx,
//             // _event_receiver: evt_rx,
//             // scheduler_notifier: oper_tx,
//             // _scheduler_client: scheduler_client,
//             // _scheduler_session: jh2,
//         })
//     }

//     // async fn run_server_session(
//     //     mut client: crate::messages::scheduler_client::SchedulerClient<tonic::transport::Channel>,
//     //     mut notifs: std::sync::mpsc::Receiver<SchedulerOperation>,
//     //     back: tokio::sync::broadcast::Sender<WorkloadEvent>,
//     // ) -> anyhow::Result<()> {
//     //     let outbound = async_stream::stream! {
//     //         loop {
//     //             println!("Waiting for something to send");
//     //             let op_msg = Self::receive_translate(&mut notifs);
//     //             println!("Got it, yielding it");
//     //             yield op_msg;
//     //         }
//     //     };

//     //     println!("Passing streeam object");
//     //     let response = client.scheduler(tonic::Request::new(outbound)).await?;
//     //     let mut inbound = response.into_inner();
//     //     println!("Passed");

//     //     while let Some(evt) = inbound.message().await? {
//     //         println!("Got something back");
//     //         if let Some(wevt) = Self::translate_evt(evt) {
//     //             back.send(wevt)?;
//     //         }
//     //     }

//     //     Ok(())
//     // }

//     // fn translate_evt(src: crate::messages::WorkloadEvent) -> Option<WorkloadEvent> {
//     //     src.message.map(|m| match m {
//     //         crate::messages::workload_event::Message::Stopped(s) => {
//     //             let wl = WorkloadId::new_from(&s.id);
//     //             let err = s.error.map(|e| Arc::new(anyhow::anyhow!("{}", e)));
//     //             WorkloadEvent::Stopped(wl, err)
//     //         },
//     //         crate::messages::workload_event::Message::UpdateFailed(s) => {
//     //             let wl = WorkloadId::new_from(&s.id);
//     //             let err = Arc::new(anyhow::anyhow!("{}", s.error));
//     //             WorkloadEvent::UpdateFailed(wl, err)
//     //         },
//     //     })
//     // }

//     // fn receive_translate(notifs: &mut std::sync::mpsc::Receiver<SchedulerOperation>) -> crate::messages::SchedulerOperation {
//     //     let op = notifs.recv().unwrap_or(SchedulerOperation::Stop);

//     //     let m = match op {
//     //         SchedulerOperation::Stop =>
//     //             crate::messages::scheduler_operation::Message::Stop(crate::messages::Stop { ignore: 0 }),
//     //         SchedulerOperation::WorkloadChanged(id) =>
//     //             crate::messages::scheduler_operation::Message::WorkloadChanged(crate::messages::WorkloadChanged { id: id.to_string() }),
//     //     };

//     //     crate::messages::SchedulerOperation {
//     //         message: Some(m),
//     //     }
//     // }

//     pub fn set_workload(&mut self, workload: &WorkloadId, spec: WorkloadSpec) -> anyhow::Result<()> {
//         // self.store.write().unwrap().set_workload(workload, spec);
//         // let oper = SchedulerOperation::WorkloadChanged(workload.clone());
//         // self.scheduler_notifier.send(oper)?;
//         Ok(())
//     }

//     pub fn remove_workload(&mut self, workload: &WorkloadId) -> anyhow::Result<()> {
//         // self.store.write().unwrap().remove_workload(workload);
//         // let oper = SchedulerOperation::WorkloadChanged(workload.clone());
//         // self.scheduler_notifier.send(oper)?;
//         Ok(())
//     }

//     pub async fn shutdown(&mut self) -> anyhow::Result<()> {
//         // self.scheduler_notifier.send(SchedulerOperation::Stop)?;
//         // (&mut self.scheduler).await?;
//         Ok(())
//     }

//     pub fn notifications(&self) -> tokio::sync::broadcast::Receiver<WorkloadEvent> {
//         // self.event_sender.subscribe()
//         todo!()
//     }
// }
