//! Publish-Subscribe Pattern Example
//!
//! This example demonstrates how to implement the Publish-Subscribe (Pub/Sub) pattern using Actors.
//!
//! - `TopicActor`: Acts as the central hub for messages, maintaining a list of subscribers.
//! - `SubscriberActor`: Subscribes to the `TopicActor` and receives broadcast messages.
//!
//! Process flow:
//! 1. Multiple `SubscriberActor`s start and send a `Subscribe` message to the `TopicActor`,
//!    containing their own `Outbox`.
//! 2. The `TopicActor` stores these `Outbox` handles.
//! 3. When a client wishes to leave, it sends an `Unsubscribe` message to the `TopicActor`.
//! 4. Upon receiving the `Unsubscribe` message, the `TopicActor` removes the corresponding `Outbox` from its list.
//! 5. When the `TopicActor` receives a `Publish` message, it iterates over the current list of active subscribers and broadcasts the message.
//!
//! This demonstrates:
//! - One-to-many message distribution.
//! - How Actor state can dynamically manage handles to other Actors (`Outbox`).
//! - Managing subscription lifecycles via explicit messages (instead of relying on send failures), which is a more robust pattern.

use acty::{Actor, ActorExt, AsyncClose, UnboundedOutbox};
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Subscriber Actor, which receives and prints messages from a topic.
struct Subscriber {
    id: usize,
    /// Used via Arc<AtomicUsize> to verify the number of received messages in the test
    msg_count: Arc<AtomicUsize>,
}

impl Actor for Subscriber {
    type Message = String;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        let mut inbox = pin!(inbox);
        println!("[Subscriber {}]: Online and listening.", self.id);
        while let Some(msg) = inbox.next().await {
            println!("[Subscriber {}]: Received message: '{}'", self.id, msg);
            self.msg_count.fetch_add(1, Ordering::SeqCst);
        }
        println!("[Subscriber {}]: Shutting down.", self.id);
    }
}

/// Topic Actor, responsible for broadcasting messages to all subscribers.
struct TopicActor;

/// Message types for the Topic Actor.
enum TopicMessage {
    /// Broadcasts a message to all subscribers
    Publish(String),
    /// Subscribes to the topic
    Subscribe {
        id: usize,
        subscriber: UnboundedOutbox<String>,
    },
    /// Unsubscribes from the topic
    Unsubscribe(usize),
}

impl Actor for TopicActor {
    type Message = TopicMessage;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        let mut inbox = pin!(inbox);
        // State: stores the ID and the Outbox handle for each subscriber
        let mut subscribers: HashMap<usize, UnboundedOutbox<String>> = HashMap::new();
        println!("[Topic]: Online.");

        while let Some(msg) = inbox.next().await {
            match msg {
                TopicMessage::Publish(text) => {
                    println!("[Topic]: Publishing message: '{}'", text);
                    // Broadcast to all current subscribers
                    for outbox in subscribers.values() {
                        // If sending fails, it's okay, as the subscriber might have closed
                        // just before receiving the message.
                        // In this pattern, we primarily rely on the Unsubscribe message to manage the list.
                        let _ = outbox.send(text.clone());
                    }
                }
                TopicMessage::Subscribe { id, subscriber } => {
                    println!("[Topic]: New subscription from ID {}.", id);
                    subscribers.insert(id, subscriber);
                }
                TopicMessage::Unsubscribe(id) => {
                    println!("[Topic]: Unsubscribe request for ID {}.", id);
                    if subscribers.remove(&id).is_some() {
                        println!("[Topic]: Subscriber {} removed.", id);
                    }
                }
            }
        }
        println!("[Topic]: Shutting down.");
    }
}

#[tokio::main]
async fn main() {
    let topic = TopicActor.start();

    // Launch and subscribe three Subscribers
    let mut subscriber_outboxes = Vec::new();
    let mut msg_counts = Vec::new();

    for i in 1..=3 {
        let count = Arc::new(AtomicUsize::new(0));
        let subscriber = Subscriber {
            id: i,
            msg_count: count.clone(),
        };
        let outbox = subscriber.start();

        // Send the subscription request
        topic
            .send(TopicMessage::Subscribe {
                id: i,
                subscriber: outbox.clone(),
            })
            .unwrap();

        subscriber_outboxes.push(outbox);
        msg_counts.push(count);
    }

    // Wait for subscriptions to complete
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Publish the first message
    topic
        .send(TopicMessage::Publish("Hello everyone!".to_string()))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Close one subscriber
    println!("\nClosing subscriber 2...\n");
    // 1. First, send the unsubscribe message
    let subscriber_2_id = 2;
    topic
        .send(TopicMessage::Unsubscribe(subscriber_2_id))
        .unwrap();

    // 2. Then, close the Actor
    let sub2_outbox = subscriber_outboxes.remove(1); // sub2 is at index 1
    sub2_outbox.close().await;

    // Publish the second message
    topic
        .send(TopicMessage::Publish("Is anyone still here?".to_string()))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Close all remaining connections
    topic.close().await;
    for outbox in subscriber_outboxes {
        outbox.close().await;
    }

    // Verification
    println!("\n--- Verification ---");
    // Subscriber 1 (index 0) should get 2 messages
    assert_eq!(msg_counts[0].load(Ordering::SeqCst), 2);
    println!(
        "Subscriber 1 received {} messages. (Correct)",
        msg_counts[0].load(Ordering::SeqCst)
    );

    // Subscriber 2 (original index 1) should get only 1 message
    assert_eq!(msg_counts[1].load(Ordering::SeqCst), 1);
    println!(
        "Subscriber 2 received {} message. (Correct)",
        msg_counts[1].load(Ordering::SeqCst)
    );

    // Subscriber 3 (original index 2) should get 2 messages
    assert_eq!(msg_counts[2].load(Ordering::SeqCst), 2);
    println!(
        "Subscriber 3 received {} messages. (Correct)",
        msg_counts[2].load(Ordering::SeqCst)
    );
}
