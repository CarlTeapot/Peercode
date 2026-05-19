use crdt_core::types::ClientId;
use crdt_core::Document;
use log::info;
use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::state::document::client::DocSender;
use crate::state::document::handlers::{local, remote, snapshot};
use crate::state::document::state::DocState;
use crate::state::document::types::{DocOp, DOC_CHANNEL_CAPACITY};

pub fn spawn(client_id: ClientId, app: AppHandle) -> DocSender {
    let (tx, rx) = mpsc::channel(DOC_CHANNEL_CAPACITY);
    let actor = DocActor::new(Document::new(client_id), rx, app);
    tauri::async_runtime::spawn(actor.run());
    tx
}

struct DocActor {
    state: DocState,
    rx: mpsc::Receiver<DocOp>,
    app: AppHandle,
}

impl DocActor {
    fn new(doc: Document, rx: mpsc::Receiver<DocOp>, app: AppHandle) -> Self {
        DocActor {
            state: DocState::new(doc),
            rx,
            app,
        }
    }

    async fn run(mut self) {
        info!("doc actor started");
        while let Some(op) = self.rx.recv().await {
            self.dispatch(op);
        }
        info!("doc actor stopped");
    }

    fn dispatch(&mut self, op: DocOp) {
        match op {
            DocOp::LocalInsert {
                position,
                content,
                base_seq,
                reply,
            } => {
                let result =
                    local::handle_local_insert(&mut self.state, position, &content, base_seq);
                let _ = reply.send(result);
            }
            DocOp::LocalDelete {
                position,
                length,
                base_seq,
                reply,
            } => {
                let result =
                    local::handle_local_delete(&mut self.state, position, length, base_seq);
                let _ = reply.send(result);
            }
            DocOp::LocalReplace {
                position,
                delete_length,
                content,
                base_seq,
                reply,
            } => {
                let result = local::handle_local_replace(
                    &mut self.state,
                    position,
                    delete_length,
                    &content,
                    base_seq,
                );
                let _ = reply.send(result);
            }

            DocOp::ApplyRemoteOp { op } => {
                remote::handle_remote_op(&mut self.state, &self.app, op);
            }
            DocOp::ApplyRemoteSnapshot { snap } => {
                snapshot::handle_remote_snapshot(&mut self.state, &self.app, snap);
            }
            DocOp::GetSnapshot { reply } => {
                let _ = reply.send(self.state.doc.to_snapshot());
            }
            DocOp::DocumentReplace { doc, reply } => {
                snapshot::handle_replace(&mut self.state, &self.app, *doc);
                let _ = reply.send(());
            }
            DocOp::GetText { reply } => {
                let _ = reply.send(self.state.doc.get_text());
            }
            DocOp::GetClientId { reply } => {
                let _ = reply.send(self.state.doc.client_id);
            }
            #[cfg(debug_assertions)]
            DocOp::DebugLinkedList { reply } => {
                let _ = reply.send(self.state.doc.debug_linked_list());
            }
        }
    }
}
