//! Defines the [`Launch`] trait, a generic strategy abstraction for starting Actors.
//!
//! This module provides an extensible mechanism to customize the Actor startup process.
//! Most users will directly use the convenience methods provided by
//! [`ActorExt`](crate::ActorExt) (such as `.start()`), which are themselves
//! concrete implementations of a [`Launch`] strategy.
//!
//! ## Motivation
//!
//! The primary goal of the `Launch` trait is to decouple **the Actor's business logic**
//! (`Actor` trait) from **the Actor's startup and runtime environment configuration**
//! (e.g., what type of message channel to use, how to spawn the `tokio` task, etc.).
//!
//! By implementing the `Launch` trait, you can create custom launchers for:
//! - Using different MPSC Channel implementations (e.g., `flume` or `crossbeam`).
//! - Executing setup or logging before the Actor starts.
//! - Configuring specific properties of the `tokio` task (e.g., using `spawn_blocking`).
//! - Returning a custom handle instead of just an `Outbox`.

use crate::Actor;

/// A trait that encapsulates the strategy for launching an Actor.
///
/// A type implementing this trait defines how to create the message channel (mailbox)
/// for an Actor, how to spawn the Actor as an asynchronous task, and what handle
/// to return for interacting with that Actor.
///
/// This is an application of the "Strategy Pattern," allowing users to inject
/// custom startup behavior. Typically, a `Launch` instance is passed to the
/// [`ActorExt::with`](crate::ActorExt::with) method to launch the Actor.
///
/// ## Generic Associated Type (GAT)
///
/// Note the use of `type Result<A>`. This is a Generic Associated Type, which
/// allows the type of the launch result to depend on the concrete Actor type `A`
/// being launched.
///
/// ## Example: A Custom Launcher
///
/// The following example creates a `CustomLauncher` that prints a log message
/// upon starting the Actor and uses a fixed bounded channel capacity.
///
/// ```
/// use acty::{Actor, ActorExt, AsyncClose, Launch, BoundedOutbox};
/// use futures::{Stream, StreamExt};
/// use std::pin::pin;
/// use std::marker::PhantomData;
/// use tokio_stream::wrappers::ReceiverStream;
///
/// // 1. Define a simple Actor
/// struct MySimpleActor;
/// impl Actor for MySimpleActor {
///
///     type Message = u32;
///
///     async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
///         let count = inbox.count().await;
///         println!("MySimpleActor processed {} messages and is finishing.", count);
///     }
/// }
///
/// // 2. Define the custom launcher
/// struct CustomLauncher<M> {
///     buffer_size: usize,
///     _phantom: PhantomData<M>
/// }
///
/// // 3. Implement the Launch trait for the launcher
/// impl<M: Send + 'static> Launch for CustomLauncher<M> {
///
///     type Message = M;
///     // Returns a Bounded Outbox after launch
///     type Result<A> = BoundedOutbox<Self::Message>;
///
///     fn launch<A>(self, actor: A) -> Self::Result<A>
///     where
///         A: Actor<Message = Self::Message>,
///     {
///         println!("Using CustomLauncher to start an actor!");
///
///         let (sender, receiver) = tokio::sync::mpsc::channel(self.buffer_size);
///         let inbox_stream = ReceiverStream::new(receiver);
///         let join_handle = tokio::spawn(actor.run(inbox_stream));
///
///         BoundedOutbox::new(sender, join_handle)
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let actor = MySimpleActor;
///     let launcher = CustomLauncher { buffer_size: 5, _phantom: PhantomData };
///
///     // 4. Use actor.with() and the custom launcher to launch the Actor
///     let outbox = actor.with(launcher);
///
///     outbox.send(10).await.unwrap();
///     outbox.send(20).await.unwrap();
///
///     // Close the outbox so the Actor can finish
///     outbox.close().await;
/// }
/// ```
pub trait Launch: Send + Sized {
    /// The type of message handled by the launched Actor.
    type Message: Send;

    /// The resulting handle type returned after launching the Actor.
    /// This is a Generic Associated Type (GAT) which depends on the Actor type `A`.
    type Result<A>;

    /// Executes the launch strategy.
    ///
    /// This method is responsible for setting up the communication channels,
    /// spawning the Actor task, and constructing the return handle.
    fn launch<A>(self, actor: A) -> Self::Result<A>
    where
        A: Actor<Message = Self::Message>;
}
