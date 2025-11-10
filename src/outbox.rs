//! Defines the handle for interacting with a running Actor: [`Outbox`].
//!
//! When an Actor is launched using methods like [`ActorExt::start`](crate::ActorExt::start),
//! an `Outbox` is returned. This `Outbox` is the sole mechanism for communicating with the Actor.
//!
//! ## Dual Responsibilities of Outbox
//!
//! An `Outbox` serves two primary functions:
//!
//! 1.  **Sending Messages**: Through its `Deref` implementation, the `Outbox` behaves
//!     like the underlying `tokio::sync::mpsc::Sender`. You can directly call
//!     `.send()` on the `Outbox` instance to send messages to the Actor.
//!
//! 2.  **Managing Lifecycle**: The `Outbox` internally holds a shared handle to the
//!     Actor task (`Arc<JoinHandle>`). When the last `Outbox` clone is dropped or
//!     [`Outbox::close`] is called, it triggers the Actor's graceful shutdown process.
//!     This "sender-driven" lifecycle management is a core design principle of the `acty` framework.
//!
//! ## Cloning and Ownership
//!
//! The `Outbox` is clonable (`Clone`). You can create multiple `Outbox` instances and
//! distribute them to different parts of your program; they all point to the same Actor.
//! The Actor will continue running until *all* `Outbox` instances pointing to it are dropped.
//! The `Outbox::close().await` method is designed to only truly wait for the Actor task
//! to finish when called on the final `Outbox` instance.
//!
//! ## Example
//!
//! ```
//! use acty::{Actor, ActorExt, AsyncClose, UnboundedOutbox};
//! use futures::{Stream, StreamExt};
//! use std::pin::pin;
//!
//! // A simple Actor that prints received messages
//! struct PrinterActor;
//!
//! impl Actor for PrinterActor {
//!     type Message = String;
//!     async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
//!         let mut inbox = pin!(inbox);
//!         while let Some(msg) = inbox.next().await {
//!             println!("Actor received: {}", msg);
//!         }
//!         println!("Actor is shutting down.");
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // Launch the Actor and get the Outbox
//!     let outbox: UnboundedOutbox<String> = PrinterActor.start();
//!
//!     // Clone the Outbox to simulate multiple senders
//!     let outbox_clone = outbox.clone();
//!
//!     // Send a message using the original Outbox
//!     outbox.send("Message from original".to_string()).unwrap();
//!
//!     // Send a message using the cloned Outbox
//!     tokio::spawn(async move {
//!         outbox_clone.send("Message from clone".to_string()).unwrap();
//!         // The clone is dropped here, but the Actor will not stop because the original outbox still exists
//!     }).await.unwrap();
//!
//!     // Close the original outbox. Since it is the last handle,
//!     // this will close the channel and wait for the Actor task to complete.
//!     outbox.close().await;
//!
//!     println!("Main function finished.");
//! }
//! ```

use crate::close::AsyncClose;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// An internal, reference-counted handle for the Actor task.
///
/// It wraps an `Arc<JoinHandle<()>>`, allowing multiple [`Outbox`] instances
/// to share a reference to the same Actor `tokio` task.
///
/// The key logic is in `wait_for_completion`: it only truly `await`s the
/// `JoinHandle` if it is the last strong reference to the `Arc`, ensuring
/// the Actor cleanup work happens exactly once.
#[derive(Debug, Clone)]
pub struct ActorHandle {
    join_handle: Arc<JoinHandle<()>>,
}

impl ActorHandle {
    /// Creates a new `ActorHandle` from a `JoinHandle`.
    fn new(join_handle: JoinHandle<()>) -> Self {
        Self {
            join_handle: Arc::new(join_handle),
        }
    }

    /// Waits for the Actor task to complete, but only if this is the last handle.
    ///
    /// This method uses `Arc::get_mut` to check if it is the unique owner of the `Arc`.
    /// If it is, it consumes the `JoinHandle` and `await`s it.
    /// If not, it returns immediately, leaving the responsibility of awaiting the task
    /// to the last remaining `ActorHandle`.
    async fn wait_for_completion(mut self) {
        if let Some(join_handle) = Arc::get_mut(&mut self.join_handle) {
            join_handle.await.unwrap();
        }
    }
}

/// A handle for interacting with a running Actor.
///
/// An `Outbox` combines a message sender (`sender`) and an [`ActorHandle`].
/// It provides the ability to send messages to the Actor and is responsible
/// for managing its lifecycle.
///
/// It is clonable and thread-safe (`Send + Sync`).
#[derive(Debug, Clone)]
pub struct Outbox<S> {
    sender: S,
    handle: ActorHandle,
}

impl<S> Outbox<S> {
    /// Creates a new `Outbox` using a sender and a `JoinHandle`.
    ///
    /// This method is typically called internally by the `acty` framework,
    /// for example, within [`ActorExt::start`](crate::ActorExt::start).
    pub fn new(sender: S, join_handle: JoinHandle<()>) -> Self {
        Self {
            sender,
            handle: ActorHandle::new(join_handle),
        }
    }
}

impl<S: AsyncClose> AsyncClose for Outbox<S> {
    /// Gracefully closes the connection to the Actor and waits for its completion.
    ///
    /// This method performs two key operations:
    /// 1. It immediately closes the message sending channel (`sender`). This signals
    ///    the Actor's `inbox` stream that no new messages will arrive, allowing it
    ///    to finish its current work and exit its main loop.
    /// 2. If this is the last `Outbox` instance, it asynchronously waits for the
    ///    Actor's `tokio` task to complete.
    ///
    /// This is the recommended way to perform a graceful shutdown.
    async fn close(self) {
        // First, close the sender end to signal the Actor's inbox stream to end.
        self.sender.close().await;
        // Then, wait for the Actor task to complete (only if this is the last handle).
        self.handle.wait_for_completion().await;
    }
}

/// Allows `Outbox<S>` to be used as if it were its internal `S` (the sender).
///
/// This `Deref` implementation is crucial for `acty`'s ergonomics. It allows you
/// to directly call sender methods, such as `send()` or `try_send()`, on the
/// `Outbox` instance without needing to access the `.sender` field.
///
/// ```ignore
/// // Instead of writing this:
/// // outbox.sender.send(message).unwrap();
///
/// // You can write this directly:
/// outbox.send(message).unwrap();
/// ```
impl<S> Deref for Outbox<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}

/// A type alias for an `Outbox` that uses `tokio`'s **bounded** MPSC channel sender.
///
/// The `send` operation will asynchronously wait when the channel is full.
/// This provides a back-pressure mechanism.
pub type BoundedOutbox<T> = Outbox<tokio::sync::mpsc::Sender<T>>;

/// A type alias for an `Outbox` that uses `tokio`'s **unbounded** MPSC channel sender.
///
/// The `send` operation is synchronous and never blocks.
/// **Caution**: If message producers consistently outpace consumers, this can lead to unbounded memory growth.
pub type UnboundedOutbox<T> = Outbox<tokio::sync::mpsc::UnboundedSender<T>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Actor;
    use futures::{Stream, StreamExt};
    use tokio::sync::{mpsc, oneshot};
    use tokio_stream::wrappers::UnboundedReceiverStream;

    // Actor Run logic: waits for start_signal to confirm startup, waits for exit_signal to confirm exit.
    // This allows us to precisely track the Actor's lifecycle for testing Outbox reference counting.
    struct LifecycleActor {
        // Actor startup signal
        start_tx: oneshot::Sender<()>,

        // Actor exit signal
        exit_tx: oneshot::Sender<()>,
    }

    impl Actor for LifecycleActor {
        type Message = u32;

        async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
            // Immediately send the startup signal
            self.start_tx.send(()).unwrap();

            // Exhaust the inbox
            let _ = inbox.for_each(|_| async {}).await;

            // Finally, send the exit signal
            self.exit_tx.send(()).unwrap_or(());
        }
    }

    // Launches a controlled LifecycleActor
    fn launch_controlled_actor() -> (
        Outbox<mpsc::UnboundedSender<u32>>,
        oneshot::Receiver<()>,
        oneshot::Receiver<()>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (start_tx, start_rx) = oneshot::channel();
        let (exit_tx, exit_rx) = oneshot::channel();

        let actor = LifecycleActor { start_tx, exit_tx };
        let join_handle = tokio::spawn(actor.run(UnboundedReceiverStream::new(receiver)));

        let outbox = Outbox::new(sender, join_handle);

        (outbox, start_rx, exit_rx)
    }

    #[tokio::test]
    async fn test_outbox_deref_to_sender() {
        let (outbox, start_rx, _) = launch_controlled_actor();

        // Wait for the Actor to start
        start_rx.await.unwrap();

        // Verify Deref implementation: can directly call send
        outbox.send(42).unwrap();

        // Cleanup
        outbox.close().await;
    }

    #[tokio::test]
    async fn test_single_outbox_waits_for_completion() {
        let (outbox, start_rx, exit_rx) = launch_controlled_actor();

        // Wait for the Actor to start
        start_rx.await.unwrap();

        // Call close().await, expecting it to wait for the task to complete
        outbox.close().await;

        // Check if the Actor has exited
        assert!(exit_rx.await.is_ok());
    }

    #[tokio::test]
    async fn test_cloned_outbox_only_last_one_waits() {
        let (outbox, start_rx, mut exit_rx) = launch_controlled_actor();

        // Wait for the Actor to start
        start_rx.await.unwrap();

        // 1. Clone the Outbox
        let clone1 = outbox.clone();
        let clone2 = outbox;

        // ---- First close: not the last reference ----

        // clone1 calls close(). Since clone2 still holds the ActorHandle, it should not wait
        clone1.close().await;

        // The Actor should not have exited yet
        assert!(exit_rx.try_recv().is_err());

        // ---- Second close: the last reference ----

        // clone2 calls close(). Since it's the last Arc reference, it should await the JoinHandle
        clone2.close().await;

        // Verify that the second close caused the Actor to exit
        assert!(
            exit_rx.await.is_ok(),
            "Second close should not wait indefinitely if Actor is already finished."
        );
    }

    #[tokio::test]
    async fn test_outbox_release_closes_channel() {
        let (sender, mut receiver) = mpsc::unbounded_channel::<i32>();

        // Create a dummy JoinHandle to simulate the Actor task (not important, just need the structure)
        let handle = tokio::spawn(async {});
        let outbox = Outbox::new(sender, handle);

        // Send a message
        outbox.send(1).unwrap();

        // Release the Outbox
        outbox.close().await;

        // Verify the Receiver can still get the message already sent
        assert_eq!(receiver.recv().await, Some(1));

        // Verify the Receiver gets None, indicating all Senders are closed
        assert_eq!(receiver.recv().await, None);
    }
}
