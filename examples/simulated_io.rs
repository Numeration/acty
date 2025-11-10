//! Simulated Asynchronous I/O Actor Example
//!
//! This example demonstrates how an Actor handles tasks that require asynchronous I/O.
//! We create a `DbActor` to simulate a key-value database.
//!
//! - The `Set` operation simulates a time-consuming write (`tokio::time::sleep`).
//! - The `Get` operation simulates a time-consuming read.
//!
//! This illustrates:
//! - How an Actor can safely `.await` asynchronous operations internally without blocking
//!   the processing of other messages (provided the Actor's logic allows it).
//! - How to use an Actor to encapsulate and serialize access to shared resources
//!   (like a database connection or in-memory state), thus avoiding the need for `Mutex`.

use acty::{Actor, ActorExt, AsyncClose};
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::pin;
use std::time::Duration;
use tokio::sync::oneshot;

/// The Actor simulating a database
struct DbActor;

/// Message types for database operations
enum DbMessage {
    /// Sets a key-value pair
    Set {
        key: String,
        value: String,
        responder: oneshot::Sender<()>,
    },
    /// Retrieves the value corresponding to a key
    Get {
        key: String,
        responder: oneshot::Sender<Option<String>>,
    },
}

impl Actor for DbActor {
    type Message = DbMessage;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        let mut inbox = pin!(inbox);

        // Actor state: an in-memory HashMap
        let mut storage = HashMap::new();

        println!("DbActor: Online. Ready to serve requests.");

        while let Some(msg) = inbox.next().await {
            match msg {
                DbMessage::Set {
                    key,
                    value,
                    responder,
                } => {
                    println!(
                        "DbActor: Received SET for key '{}'. Simulating write delay...",
                        key
                    );
                    // Simulate a time-consuming database write
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    storage.insert(key.clone(), value);
                    println!("DbActor: Key '{}' set successfully.", key);
                    responder.send(()).unwrap_or(());
                }
                DbMessage::Get { key, responder } => {
                    println!(
                        "DbActor: Received GET for key '{}'. Simulating read delay...",
                        key
                    );
                    // Simulate a time-consuming database read
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let value = storage.get(&key).cloned();
                    println!("DbActor: Responding for key '{}'.", key);
                    responder.send(value).unwrap_or(());
                }
            }
        }
        println!("DbActor: Shutting down.");
    }
}

#[tokio::main]
async fn main() {
    let db = DbActor.start();

    // Task 1: Set a value
    let (tx1, rx1) = oneshot::channel();
    let set_msg = DbMessage::Set {
        key: "name".to_string(),
        value: "acty".to_string(),
        responder: tx1,
    };
    db.send(set_msg).unwrap_or(());
    // Wait for the write to complete
    rx1.await.unwrap();
    println!("Main: SET operation confirmed.\n");

    // Task 2: Get an existing value
    let (tx2, rx2) = oneshot::channel();
    let get_msg_1 = DbMessage::Get {
        key: "name".to_string(),
        responder: tx2,
    };
    db.send(get_msg_1).unwrap_or(());
    let value1 = rx2.await.unwrap();
    println!("Main: Got value: {:?}", value1);
    assert_eq!(value1, Some("acty".to_string()));
    println!();

    // Task 3: Get a non-existent value
    let (tx3, rx3) = oneshot::channel();
    let get_msg_2 = DbMessage::Get {
        key: "version".to_string(),
        responder: tx3,
    };
    db.send(get_msg_2).unwrap_or(());
    let value2 = rx3.await.unwrap();
    println!("Main: Got value: {:?}", value2);
    assert_eq!(value2, None);
    println!();

    db.close().await;
    println!("Main: Actor closed gracefully.");
}
