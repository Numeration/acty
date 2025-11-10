use crate::close::AsyncClose;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct ActorHandle {
    join_handle: Arc<JoinHandle<()>>,
}

impl ActorHandle {
    fn new(join_handle: JoinHandle<()>) -> Self {
        Self {
            join_handle: Arc::new(join_handle),
        }
    }

    async fn wait_for_completion(mut self) {
        if let Some(join_handle) = Arc::get_mut(&mut self.join_handle) {
            join_handle.await.unwrap();
        }
    }
}

#[derive(Debug, Clone)]
pub struct Outbox<S> {
    sender: S,
    handle: ActorHandle,
}

impl<S> Outbox<S> {
    pub fn new(sender: S, join_handle: JoinHandle<()>) -> Self {
        Self { sender, handle: ActorHandle::new(join_handle) }
    }
}

impl<S: AsyncClose> Outbox<S> {
    pub async fn close(self) {
        self.sender.close().await;
        self.handle.wait_for_completion().await;
    }
}

impl<S> Deref for Outbox<S> {

    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}

pub type BoundedOutbox<T> = Outbox<tokio::sync::mpsc::Sender<T>>;

pub type UnboundedOutbox<T> = Outbox<tokio::sync::mpsc::UnboundedSender<T>>;
