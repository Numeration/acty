use tokio::task::JoinHandle;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};
use crate::{Actor, BoundedOutbox, Inbox, UnboundedOutbox};

pub trait Start<T: Send + 'static>: Actor<T> {
    fn start_with(self, inbox: impl Inbox<Item = T>) -> JoinHandle<()> {
        tokio::spawn(self.run(inbox))
    }

    fn start_with_mailbox_capacity(self, mailbox_capacity: usize) -> BoundedOutbox<T> {
        let (sender, receiver) = tokio::sync::mpsc::channel(mailbox_capacity);
        let receiver_stream = ReceiverStream::new(receiver);
        BoundedOutbox::new(sender, self.start_with(receiver_stream))
    }

    fn start(self) -> UnboundedOutbox<T> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let receiver_stream = UnboundedReceiverStream::new(receiver);
        UnboundedOutbox::new(sender, self.start_with(receiver_stream))
    }
}

impl<T: Send + 'static, A: Actor<T>> Start<T> for A {}
