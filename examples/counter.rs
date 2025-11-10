//! Counter Actor Example
//!
//! This example demonstrates a simple counter Actor, illustrating the following features:
//! - Actor state management
//! - Asynchronous message processing
//! - Synchronous instruction response (oneshot channel)
//! - Starting and gracefully closing an Actor
use acty::{Actor, ActorExt, AsyncClose};
use futures::{Stream, StreamExt};
use std::pin::pin;

/// Counter Actor
///
/// This Actor maintains the count value `value` internally, but the struct itself
/// contains no fields. The state is maintained asynchronously within the `run` method.
struct Counter;

/// Message types for the Counter Actor
///
/// The Actor updates or provides the count value by receiving these messages.
enum CounterMessage {
    /// Increments the count value
    Increment,

    /// Requests the current count value
    ///
    /// The sender receives the count result via `tokio::sync::oneshot::Receiver<u64>`
    GetValue(tokio::sync::oneshot::Sender<u64>),
}

/// Implements the `Actor` trait for the `Counter` Actor
///
/// The Actor message type is `CounterMessage`, and it asynchronously maintains
/// the internal count value `value`.
///
/// - Iterates over the inbox to process messages
/// - Updates the state or returns the result via oneshot based on the message
impl Actor for Counter {
    type Message = CounterMessage;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        // Pin the inbox to the stack for asynchronous iteration
        let mut inbox = pin!(inbox);

        // The count value (Actor state)
        let mut value = 0;

        // Asynchronously process each message
        while let Some(msg) = inbox.next().await {
            match msg {
                // Increment the counter
                CounterMessage::Increment => value += 1,

                // Return the current count via oneshot
                // If sending fails (e.g., the Actor has already finished), ignore the error
                CounterMessage::GetValue(tx) => tx.send(value).unwrap_or(()),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Launch the counter Actor
    let counter = Counter.start();

    // Send three increment instructions
    counter.send(CounterMessage::Increment).unwrap_or(());
    counter.send(CounterMessage::Increment).unwrap_or(());
    counter.send(CounterMessage::Increment).unwrap_or(());

    // Create a oneshot channel to retrieve the current count value
    let (rx, tx) = tokio::sync::oneshot::channel();
    counter.send(CounterMessage::GetValue(rx)).unwrap_or(());

    // Asynchronously wait for the result and print it
    println!("count: {}", tx.await.unwrap());

    // Close the outbox and wait for the Actor to finish
    counter.close().await;
}
