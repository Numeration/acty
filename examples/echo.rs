//! Echo Actor Example
//!
//! This example demonstrates how to implement the simplest possible Actor,
//! illustrating the following features:
//! - Implementing the Actor trait
//! - Processing Inbox messages
//! - Sending messages to the Actor
//! - Correctly releasing the Outbox
use acty::{Actor, ActorExt, AsyncClose};
use futures::{Stream, StreamExt};
use std::pin::pin;

/// The simplest Echo Actor
///
/// This Actor processes messages of type `String` and prints them to standard output.
/// Note: This Actor has no state fields, demonstrating only the basic usage.
struct Echo;

/// Implements the `Actor` trait, with message type `String`
///
/// When a message is received in the Inbox, it is printed line by line to standard output.
impl Actor for Echo {
    type Message = String;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        // Pin the inbox to the stack for asynchronous iteration.
        // If it needed to be passed across function boundaries, Box::pin could be used
        // to pin it to the heap (with a small allocation overhead).
        let mut inbox = pin!(inbox);

        // Asynchronously iterate through every message in the inbox
        while let Some(msg) = inbox.next().await {
            println!("echo: {}", msg);
        }
    }
}

#[tokio::main]
async fn main() {
    // Launch the actor and obtain the associated outbox
    let echo = Echo.start();

    // Send messages to the Actor
    // send returns a Result, returning Err if the Actor has shut down
    // We use unwrap_or(()) here to ignore potential errors
    echo.send("Hello".to_string()).unwrap_or(());
    echo.send("World".to_string()).unwrap_or(());
    echo.send("!".to_string()).unwrap_or(());

    // Close the outbox, unbinding it from the actor.
    // This ensures the actor finishes naturally after processing all messages.
    echo.close().await;
}
