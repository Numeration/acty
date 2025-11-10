use futures::Stream;
use crate::launch::Launch;
use crate::{Actor, BoundedOutbox, UnboundedOutbox};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};

pub trait ActorExt: Actor {
    fn with<L>(self, launch: L) -> L::Result<Self>
    where
        L: Launch<Message = Self::Message>
    {
        launch.launch(self)
    }

    fn start_with<I>(self, inbox: I) -> JoinHandle<()>
    where
        I: Stream<Item = Self::Message> + Send + 'static
    {
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
