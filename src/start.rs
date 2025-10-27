use tokio::task::JoinHandle;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};
use crate::{Actor, BoundedOutbox, Inbox, UnboundedOutbox};
use crate::launch::Launch;

pub trait ActorExt: Actor {
    
    fn with<L: Launch<Message = Self::Message>>(self, launch: L)  -> L::Result {
        launch.launch(self)
    }
    
    fn start_with(self, inbox: impl Inbox<Item = Self::Message>) -> JoinHandle<()> {
        tokio::spawn(self.run(inbox))
    }

    fn start_with_mailbox_capacity(self, mailbox_capacity: usize) -> BoundedOutbox<Self::Message> {
        let (sender, receiver) = tokio::sync::mpsc::channel(mailbox_capacity);
        let receiver_stream = ReceiverStream::new(receiver);
        BoundedOutbox::new(sender, self.start_with(receiver_stream))
    }

    fn start(self) -> UnboundedOutbox<Self::Message> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let receiver_stream = UnboundedReceiverStream::new(receiver);
        UnboundedOutbox::new(sender, self.start_with(receiver_stream))
    }
}

impl<A: Actor> ActorExt for A {}
