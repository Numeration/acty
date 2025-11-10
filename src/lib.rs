//! # Acty
//!
//! Acty is a lightweight and high-performance Actor framework built on top of Tokio.
//! It adheres to the core philosophies of "everything is an Actor" and "sender-driven lifecycle,"
//! aiming to provide a simple, safe, and easy-to-use concurrency model.
//!
//! ## Core Concepts
//!
//! - **Actor**: Any type that implements the [`Actor`] trait is an Actor. Its core is the `run` method, which contains all the business logic and state management.
//! - **Lifecycle**: An Actor's lifecycle is determined by its message senders (the `Outbox`). When all associated `Outbox` instances are dropped, the Actor's message channel (`inbox`) closes, and the Actor task gracefully terminates. This design eliminates the complexity of manually sending "stop" messages.
//! - **Messaging**: By calling the [`ActorExt::start`] method to launch an Actor, you receive an [`UnboundedOutbox`] (or [`BoundedOutbox`]). This `Outbox` is the sole handle for sending messages to the Actor.
//! - **State Management**: The Actor's state usually resides as local variables within the asynchronous scope of its `run` method, rather than as fields of the structure. This ensures cleaner state isolation and ownership management.
//!
//! ## Result Communication (Backchannel)
//!
//! An Actor can send its final computation result back to the caller through mechanisms like `oneshot` channels before its task ends.
//!
//! ### Example: An Actor that concatenates strings
//! ```rust
//! use std::pin::pin;
//! use std::fmt::Write;
//! use futures::{Stream, StreamExt};
//! use acty::{Actor, ActorExt, AsyncClose};
//!
//! // 1. Define the Actor structure. It holds a oneshot::Sender to return the result.
//! struct MyActor {
//!    result: tokio::sync::oneshot::Sender<String>,
//! }
//!
//! // 2. Implement the Actor trait, defining the message type and core logic.
//! impl Actor for MyActor {
//!     type Message = String;
//!
//!     async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
//!         // Pin the inbox to the stack for asynchronous iteration.
//!         let mut inbox = pin!(inbox);
//!
//!         // The Actor's internal state is defined and managed within the run method.
//!         let mut article = String::new();
//!         let mut count = 1;
//!
//!         // Loop through messages until the inbox closes.
//!         while let Some(msg) = inbox.next().await {
//!             write!(article, "part {}: {}\n", count, msg.as_str()).unwrap();
//!             count += 1;
//!         }
//!
//!         // When all Outboxes are dropped, the loop ends. Send the final result here.
//!         // Sending might fail if the receiver has already given up waiting, so we ignore the error.
//!         let _ = self.result.send(article);
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // 3. Create the channel for receiving the result.
//!     let (tx, rx) = tokio::sync::oneshot::channel();
//!
//!     // 4. Instantiate and launch the Actor, getting the Outbox handle.
//!     let my_actor = MyActor { result: tx }.start();
//!
//!     // 5. Send messages to the Actor. Ignore potential send errors caused by early shutdown.
//!     my_actor.send("It's a good day today.".to_string()).unwrap_or(());
//!     my_actor.send("Let's have some tea first.".to_string()).unwrap_or(());
//!
//!     // 6. Close the outbox. Since it's the last Outbox, this triggers the Actor's shutdown.
//!     // .close().await waits for the Actor task to fully complete.
//!     my_actor.close().await;
//!
//!     // 7. Wait for and retrieve the Actor's final execution result.
//!     assert_eq!(
//!         "part 1: It's a good day today.\npart 2: Let's have some tea first.\n",
//!         rx.await.expect("Actor task failed to send the result")
//!     );
//! }
//! ```
//!
//! For more examples, please see the examples directory.

mod actor;
mod close;
mod launch;
mod outbox;
mod start;

pub use {
    actor::Actor, close::AsyncClose, launch::Launch, outbox::BoundedOutbox,
    outbox::UnboundedOutbox, start::ActorExt,
};
