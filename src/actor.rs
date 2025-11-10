use futures::Stream;

/// Represents a fundamental computation unit in the `acty` framework.
///
/// An `Actor` is an entity that implements specific business logic by processing
/// an asynchronous message stream (the `inbox`).
///
/// ## Core Concept
///
/// The core of an Actor is its [`run`] method. This method contains the Actor's
/// main loop, which continuously pulls and processes messages from the `inbox`
/// until the stream ends.
///
/// ## Actor Lifecycle
///
/// An Actor's lifecycle is determined by the lifecycle of its `inbox`, which is a
/// "sender-driven" pattern. When all [`Outbox`](crate::UnboundedOutbox) instances
/// associated with the Actor (i.e., the message senders) are destroyed, the
/// `inbox` stream naturally closes. This causes the message loop in the `run`
/// method to finish, leading to the Actor's `tokio` task exiting gracefully.
/// This design avoids the need to manually send a "stop" message, making
/// lifecycle management simpler and safer.
///
/// ## State Management
///
/// An Actor's state usually exists as local variables within the asynchronous
/// scope of the `run` method. The Actor structure itself (`Self`) is often
/// only used to carry initial configuration or handles for result communication
/// (e.g., a `tokio::sync::oneshot::Sender`). When the `run` method starts,
/// `self` is consumed, and its fields can be moved into the `run` method's scope.
///
/// ## Trait Bounds: `Sized + Send + 'static`
///
/// - `Sized`: This is a standard requirement, meaning the Actor's size must be known at compile time.
/// - `Send`: Since the Actor will be moved to a new task by `tokio::spawn` (potentially on a different thread), it must be thread-safe itself.
/// - `'static`: The Actor must not contain any non-static references, ensuring it remains valid throughout its entire execution lifecycle.
///
/// The `#[trait_variant::make(Send)]` macro automatically ensures that implementors
/// of this trait satisfy the `Send` constraint.
///
/// ## Example
///
/// Here is an example of a `SummarizerActor` that receives a series of text fragments
/// and returns them concatenated into an article upon completion.
///
/// ```
/// use acty::{Actor, ActorExt, AsyncClose};
/// use futures::{Stream, StreamExt};
/// use std::pin::pin;
/// use tokio::sync::oneshot;
///
/// /// An Actor used to concatenate strings.
/// /// It holds a `oneshot::Sender` in its structure to return the final result
/// /// when the task is complete.
/// struct SummarizerActor {
///     result_sender: oneshot::Sender<String>,
/// }
///
/// /// Implement the Actor trait
/// impl Actor for SummarizerActor {
///     // This Actor handles messages of type String
///     type Message = String;
///
///     /// The Actor's main logic
///     async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
///         // Pin the inbox to the stack to enable use of .next()
///         let mut inbox = pin!(inbox);
///
///         // Initialize the Actor's internal state
///         let mut summary = String::new();
///
///         // Loop to process messages from the inbox
///         while let Some(fragment) = inbox.next().await {
///             summary.push_str(&fragment);
///             summary.push('\n');
///         }
///
///         // The loop finishes when the inbox closes (all Outboxes are dropped).
///         // At this point, send the final result via the oneshot channel.
///         // Ignore the error if the receiver has been dropped.
///         self.result_sender.send(summary).unwrap_or(());
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     // 1. Create the oneshot channel to receive the result
///     let (tx, rx) = oneshot::channel();
///
///     // 2. Create the Actor instance
///     let actor = SummarizerActor { result_sender: tx };
///
///     // 3. Launch the Actor and get an Outbox for sending messages
///     let outbox = actor.start();
///
///     // 4. Send messages to the Actor
///     outbox.send("This is the first part.".to_string()).unwrap();
///     outbox.send("This is the second part.".to_string()).unwrap();
///
///     // 5. Close the outbox, which will cause the Actor's inbox stream to end
///     outbox.close().await;
///
///     // 6. Wait for and retrieve the final result returned by the Actor
///     let result = rx.await.expect("Actor failed to send the result");
///
///     assert_eq!(result, "This is the first part.\nThis is the second part.\n");
///     println!("Final summary:\n{}", result);
/// }
/// ```
#[trait_variant::make(Send)]
pub trait Actor: Sized + 'static {
    /// The type of message the Actor can process.
    ///
    /// This type must implement `Send` to be safely transferable between threads.
    type Message: Send;

    /// The Actor's main logic entry point.
    ///
    /// The `acty` framework calls this method when the Actor is launched.
    /// The Actor instance (`self`) is consumed and moved into a new task managed by `tokio`.
    ///
    /// # Parameters
    /// - `self`: The Actor instance itself, moved into the asynchronous task.
    /// - `inbox`: An asynchronous stream from which the Actor will receive messages. The implementation should continuously consume items from this stream
    ///   until it returns `None`, which signals that all senders have closed and the Actor should terminate.
    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send);
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{StreamExt, stream};
    use std::pin::pin;
    use std::sync::{Arc, Mutex};
    use tokio::sync::oneshot;

    struct CollectorActor {
        // Use Arc<Mutex> to safely share the results between the Actor task and the test task
        results: Arc<Mutex<Vec<String>>>,
    }

    impl Actor for CollectorActor {
        type Message = String;

        async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
            let mut inbox = pin!(inbox);

            let mut collected_msgs = Vec::new();
            while let Some(msg) = inbox.next().await {
                collected_msgs.push(msg);
            }

            // Write the collected results to the shared state
            let mut results = self.results.lock().unwrap();
            *results = collected_msgs;
        }
    }

    struct ResultActor {
        // oneshot sender for returning the result
        tx: oneshot::Sender<u64>,
    }

    impl Actor for ResultActor {
        type Message = u64;

        async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
            let mut inbox = Box::pin(inbox);

            let mut sum = 0;
            while let Some(val) = inbox.next().await {
                sum += val;
            }

            // Send the final result when the task ends
            self.tx.send(sum).unwrap_or(());
        }
    }

    #[tokio::test]
    async fn test_collector_actor_receives_all_messages() {
        let results_shared = Arc::new(Mutex::new(Vec::new()));

        let actor = CollectorActor {
            results: results_shared.clone(),
        };

        // Construct a Stream as Inbox, simulating incoming messages
        let messages = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let message_stream = stream::iter(messages);

        // Spawn the Actor task
        let handle = tokio::spawn(actor.run(message_stream));

        // Wait for the Actor task to complete
        handle.await.expect("Actor task failed");

        // Verify the messages received by the Actor
        let received = results_shared.lock().unwrap();
        assert_eq!(received.len(), 3);
        assert_eq!(received[0], "A");
        assert_eq!(received[1], "B");
        assert_eq!(received[2], "C");
    }

    #[tokio::test]
    async fn test_result_actor_calculates_sum_and_returns_result() {
        let (tx, rx) = oneshot::channel();

        let actor = ResultActor { tx };

        // Construct input stream: 1, 2, 3, 4
        let input = vec![1, 2, 3, 4];
        let message_stream = stream::iter(input);

        // Spawn the Actor task
        let handle = tokio::spawn(actor.run(message_stream));

        // Wait for the task to complete
        handle.await.expect("Actor task failed");

        // Wait for the result sent back by the Actor
        let sum = rx.await.expect("Failed to receive result from Actor");

        // Verify the calculation result
        assert_eq!(sum, 1 + 2 + 3 + 4);
    }

    #[tokio::test]
    async fn test_actor_trait_bounds_and_lifecycle() {
        // Verify that the Actor can start and exit normally even without messages (empty Stream)
        let (tx, rx) = oneshot::channel();
        let actor = ResultActor { tx };

        let empty_stream = stream::empty();

        let handle = tokio::spawn(actor.run(empty_stream));

        // Wait for the task to complete
        handle.await.expect("Actor task failed on empty stream");

        // Verify the returned result is the initial value 0
        let sum = rx.await.expect("Failed to receive result");
        assert_eq!(sum, 0);
    }
}
