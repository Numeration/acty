use std::ops::{Deref, DerefMut};
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
pub struct BoundedOutbox<T> {
    sender: tokio::sync::mpsc::Sender<T>,
    join: ActorHandle,
}

impl<T> BoundedOutbox<T> {
    pub fn new(sender: tokio::sync::mpsc::Sender<T>, join_handle: JoinHandle<()>) -> Self {
        Self {
            sender,
            join: ActorHandle::new(join_handle),
        }
    }

    pub async fn detach(self) {
        drop(self.sender);
        self.join.wait_for_completion().await
    }
}

impl<T> Deref for BoundedOutbox<T> {
    type Target = tokio::sync::mpsc::Sender<T>;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}

impl<T> DerefMut for BoundedOutbox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sender
    }
}

#[derive(Debug, Clone)]
pub struct UnboundedOutbox<T> {
    sender: tokio::sync::mpsc::UnboundedSender<T>,
    join: ActorHandle,
}

impl<T> UnboundedOutbox<T> {
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<T>, join_handle: JoinHandle<()>) -> Self {
        Self {
            sender,
            join: ActorHandle::new(join_handle),
        }
    }

    pub async fn detach(self) {
        drop(self.sender);
        self.join.wait_for_completion().await
    }
}

impl<T> Deref for UnboundedOutbox<T> {
    type Target = tokio::sync::mpsc::UnboundedSender<T>;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}

impl<T> DerefMut for UnboundedOutbox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sender
    }
}
