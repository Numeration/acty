//! Provides extension methods for launching [`Actor`]s.
//!
//! This module defines the [`ActorExt`] trait, which is the primary entry point
//! for interacting with Actors. By providing a blanket implementation for all types
//! that implement [`Actor`], users can conveniently launch Actors using methods
//! like `.start()` without manually handling `tokio::spawn` and channel creation.

use crate::launch::Launch;
use crate::{Actor, BoundedOutbox, UnboundedOutbox};
use futures::Stream;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};

/// Provides convenient startup methods for types that implement [`Actor`].
///
/// This is an extension trait (extension trait) automatically provided to all Actors
/// via a blanket implementation. It encapsulates the common logic of creating a
/// message channel (mailbox), spawning a `tokio` task, and connecting the two.
///
/// The most frequently used methods are [`ActorExt::start`] and
/// [`ActorExt::start_with_mailbox_capacity`].
pub trait ActorExt: Actor {
    /// Launches the Actor using a custom launch strategy.
    ///
    /// This is a generic entry point that delegates the actual startup logic to
    /// any type implementing the [`Launch`] trait. This is useful for advanced use cases
    /// requiring custom channel types or specialized spawning logic.
    ///
    /// # Parameters
    /// - `launch`: An instance implementing the [`Launch`] trait, which defines how the Actor should be launched.
    fn with<L>(self, launch: L) -> L::Result<Self>
    where
        L: Launch<Message = Self::Message>,
    {
        launch.launch(self)
    }

    /// Launches the Actor using an existing message stream (`inbox`).
    ///
    /// This is a lower-level launch method that allows you to provide any type
    /// implementing `Stream` as the Actor's message source.
    ///
    /// # Parameters
    /// - `inbox`: The message stream from which the Actor will receive messages.
    ///
    /// # Returns
    /// Returns a `JoinHandle<()>` which you can `await` to wait for the Actor task to complete.
    fn start_with<I>(self, inbox: I) -> JoinHandle<()>
    where
        I: Stream<Item = Self::Message> + Send + 'static,
    {
        tokio::spawn(self.run(inbox))
    }

    /// Launches the Actor, creating a **bounded** message channel with the specified capacity.
    ///
    /// When the channel is full, the operation to send a message will asynchronously wait
    /// until space becomes available. This provides a back-pressure mechanism,
    /// preventing message producers from overwhelming the Actor.
    ///
    /// # Parameters
    /// - `mailbox_capacity`: The maximum capacity of the message channel.
    ///
    /// # Returns
    /// Returns a [`BoundedOutbox<Self::Message>`] for sending messages to the Actor.
    ///
    /// # Examples
    /// ```
    /// use acty::{Actor, ActorExt, AsyncClose};
    /// use futures::{Stream, StreamExt};
    /// use std::pin::pin;
    ///
    /// struct PrinterActor;
    ///
    /// impl Actor for PrinterActor {
    ///     type Message = String;
    ///     async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
    ///         let mut inbox = pin!(inbox);
    ///         while let Some(msg) = inbox.next().await {
    ///             println!("Received: {}", msg);
    ///         }
    ///         println!("Actor finished.");
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     // Launch the Actor with a mailbox capacity of 10
    ///     let outbox = PrinterActor.start_with_mailbox_capacity(10);
    ///
    ///     // Send a message
    ///     outbox.send("Hello, Actor!".to_string()).await.unwrap();
    ///
    ///     // Closing the outbox causes the Actor to finish
    ///     outbox.close().await;
    /// }
    /// ```
    fn start_with_mailbox_capacity(self, mailbox_capacity: usize) -> BoundedOutbox<Self::Message> {
        let (sender, receiver) = tokio::sync::mpsc::channel(mailbox_capacity);
        let receiver_stream = ReceiverStream::new(receiver);
        BoundedOutbox::new(sender, self.start_with(receiver_stream))
    }

    /// Launches the Actor, creating an **unbounded** message channel.
    ///
    /// This is the most common and convenient way to launch an Actor.
    ///
    /// **Caution**: An unbounded channel means sending messages will never
    /// fail (synchronously) or block (asynchronously). If message production
    /// consistently outpaces the Actor's processing speed, this can lead to
    /// unbounded memory growth.
    ///
    /// # Returns
    /// Returns an [`UnboundedOutbox<Self::Message>`] for sending messages to the Actor.
    ///
    /// # Examples
    /// ```
    /// use acty::{Actor, ActorExt, AsyncClose};
    /// use futures::{Stream, StreamExt};
    /// use std::pin::pin;
    ///
    /// struct GreeterActor;
    ///
    /// impl Actor for GreeterActor {
    ///     type Message = String;
    ///     async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
    ///         let mut inbox = pin!(inbox);
    ///         if let Some(name) = inbox.next().await {
    ///             println!("Hello, {}!", name);
    ///         }
    ///         println!("Greeter finished.");
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     // Launch the Actor using .start()
    ///     let outbox = GreeterActor.start();
    ///
    ///     // Send a message
    ///     outbox.send("World".to_string()).unwrap();
    ///
    ///     // Close the outbox and wait for the Actor to terminate
    ///     outbox.close().await;
    /// }
    /// ```
    fn start(self) -> UnboundedOutbox<Self::Message> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let receiver_stream = UnboundedReceiverStream::new(receiver);
        UnboundedOutbox::new(sender, self.start_with(receiver_stream))
    }
}

impl<A: Actor> ActorExt for A {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Actor, AsyncClose};
    use futures::StreamExt;
    use std::pin::pin;
    use tokio::sync::oneshot;

    /// A simple Actor used to count messages and return the total
    struct CountActor {
        // Sender for returning the final count
        result_tx: oneshot::Sender<u64>,
    }

    impl Actor for CountActor {
        type Message = String;

        async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
            let mut inbox = pin!(inbox);
            let mut count = 0;

            // Receive and consume all messages
            while (inbox.next().await).is_some() {
                count += 1;
            }

            // Return the total count
            self.result_tx.send(count).unwrap_or(());
        }
    }

    /// Creates a CountActor instance and returns its oneshot receiver
    fn create_count_actor() -> (CountActor, oneshot::Receiver<u64>) {
        let (tx, rx) = oneshot::channel();
        (CountActor { result_tx: tx }, rx)
    }

    #[tokio::test]
    async fn test_unbounded_start() {
        let (actor, rx) = create_count_actor();

        // Launch using start(), returns UnboundedOutbox
        let outbox = actor.start();

        // Unbounded Sender is synchronous
        outbox.send("msg1".to_string()).expect("Failed to send 1");
        outbox.send("msg2".to_string()).expect("Failed to send 2");

        // Close the Outbox, triggering Actor exit
        outbox.close().await;

        // Verify the number of messages received by the Actor
        let count = rx.await.expect("Actor did not return result");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_bounded_start() {
        let (actor, rx) = create_count_actor();

        // Launch Bounded Outbox with capacity=5
        let outbox = actor.start_with_mailbox_capacity(5);

        for i in 0..3 {
            outbox
                .send(format!("msg{}", i))
                .await
                .expect("Failed to send");
        }

        // Close the Outbox, triggering Actor exit
        outbox.close().await;

        // Verify the number of messages received by the Actor
        let count = rx.await.expect("Actor did not return result");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_bounded_start_with_backpressure() {
        // Use a channel with capacity 1, and the Actor never consumes messages,
        // to trigger capacity limits.

        struct BlockingActor {
            start_tx: oneshot::Sender<()>,
            _stop_rx: oneshot::Receiver<()>,
        }

        impl Actor for BlockingActor {
            type Message = String;

            async fn run(self, _: impl Stream<Item = Self::Message> + Send) {
                // Actor sends signal immediately upon startup, then blocks without consuming the inbox
                self.start_tx.send(()).unwrap();
                // The task will not exit until the Sender is dropped
                self._stop_rx.await.unwrap_or(());
            }
        }

        let (start_tx, start_rx) = oneshot::channel();
        let (stop_tx, stop_rx) = oneshot::channel(); // Used for cleanup assistance

        let actor = BlockingActor {
            start_tx,
            _stop_rx: stop_rx,
        };

        let outbox = actor.start_with_mailbox_capacity(1);

        // 1. Wait for the Actor to start, ensuring the channel is ready to receive
        start_rx.await.unwrap();

        // 2. Send the first message (success, occupies capacity 1)
        outbox.send("m1".to_string()).await.unwrap();

        // 3. Attempt to send the second message, which should block or wait because it's full.
        // Since tokio::mpsc::Sender::send() waits for space, we use `timeout` to verify it blocks.
        let send_future = outbox.send("m2".to_string());

        let timeout_result =
            tokio::time::timeout(tokio::time::Duration::from_millis(50), send_future).await;

        // Verify the send timed out, meaning it was blocked (backpressure is effective)
        assert!(
            timeout_result.is_err(),
            "Send should have timed out due to full channel"
        );

        // 4. Cleanup: Release the Outbox, stop the BlockingActor
        drop(stop_tx); // Release the Actor's auxiliary resource
        outbox.close().await;
    }
}
