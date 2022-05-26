use std::sync::{RwLock, Arc};

use messaging::{SchedulerOperationSender, RemoteOperationSender};
use scheduler::{LocalScheduler};
use schema::SchedulerOperation;
pub use schema::{WorkloadEvent, WorkloadId, WorkloadManifest, WorkloadOpts, WorkloadSpec, WorkloadStatus};
use store::{WorkStore, InMemoryWorkStore};

mod messaging;
mod run;
pub(crate) mod scheduler;
pub(crate) mod schema;
pub(crate) mod store;

pub struct Control {
    scheduler: tokio::task::JoinHandle<()>,
    store: Arc<RwLock<Box<dyn WorkStore + Send + Sync>>>,
    event_sender: tokio::sync::broadcast::Sender<WorkloadEvent>,  // For in memory it sorta works to have the comms directly from scheduler but WHO KNOWS
    _event_receiver: tokio::sync::broadcast::Receiver<WorkloadEvent>,
    scheduler_notifier: SchedulerOperationSender, // tokio::sync::broadcast::Sender<SchedulerOperation>,
}

impl Control {
    pub fn in_memory() -> Self {
        let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));
        let (evt_tx, evt_rx) = tokio::sync::broadcast::channel(1000);
        let (oper_tx, oper_rx) = tokio::sync::broadcast::channel(1000);
        let scheduler = LocalScheduler::new_in_process(store.clone(), &evt_tx, oper_rx);
        let jh = scheduler.start();
        Self {
            scheduler: jh,
            store,
            event_sender: evt_tx,
            _event_receiver: evt_rx,
            scheduler_notifier: SchedulerOperationSender::InProcess(oper_tx),
        }
    }

    pub fn in_memory_rpc(addr: &str) -> Self {
        let box_store: Box<dyn WorkStore + Send + Sync> = Box::new(InMemoryWorkStore::new());
        let store = Arc::new(RwLock::new(box_store));
        let (evt_tx, evt_rx) = tokio::sync::broadcast::channel(1000);
        // let (oper_tx, oper_rx) = tokio::sync::broadcast::channel(1000);
        let scheduler = LocalScheduler::new_rpc(store.clone(), &evt_tx, addr);
        let jh = scheduler.start();

        let ros = RemoteOperationSender::new(addr);

        Self {
            scheduler: jh,
            store,
            event_sender: evt_tx,
            _event_receiver: evt_rx,
            scheduler_notifier: SchedulerOperationSender::Remote(ros),
        }
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

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.scheduler_notifier.send(SchedulerOperation::Stop)?;
        (&mut self.scheduler).await?;
        Ok(())
    }

    pub fn notifications(&self) -> tokio::sync::broadcast::Receiver<WorkloadEvent> {
        self.event_sender.subscribe()
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
