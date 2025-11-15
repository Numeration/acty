# Acty: A Lightweight Actor Framework for Tokio

**English** | [简体中文](./README.cn.md)

[![Crates.io](https://img.shields.io/crates/v/acty.svg)](https://crates.io/crates/acty)
[![Docs.rs](https://docs.rs/acty/badge.svg)](https://docs.rs/acty)

**Acty** is a high-performance, extremely lightweight Actor framework built on top of [Tokio](https://tokio.rs/). It aims to provide a simple, safe, and ergonomic concurrency model for the Rust asynchronous ecosystem.

## Core Philosophy

Acty's design revolves around several core principles:

*   **Minimalism**: Only a single `Actor` trait needs to be implemented. No complex lifecycle hooks, no macro magic.
*   **Sender-Driven Lifecycle**: An Actor's lifespan is entirely controlled by its message senders (the `Outbox`). When all `Outbox` instances are dropped, the Actor automatically and gracefully shuts down. This eliminates the need for manual Actor termination management, fundamentally avoiding resource leaks.
*   **State Isolation**: The Actor's state exists as local variables within its core `run` method, rather than as struct fields. This makes state management clear and simple, and inherently prevents data races.
*   **Seamless Tokio Integration**: Acty is built upon fundamental Tokio components like `mpsc` channels and asynchronous `Stream`s, allowing for easy composition with any library in the Tokio ecosystem (e.g., `hyper`, `reqwest`, `tonic`).

## Features

*   **Lightweight**: The core API consists of only a few key traits (`Actor`, `ActorExt`, `AsyncClose`).
*   **Type Safe**: Leverages Rust's type system to ensure correct Actor message passing.
*   **High Performance**: Directly uses Tokio's MPSC channels underneath, resulting in minimal performance overhead.
*   **Supports Bounded and Unbounded Channels**: Easily launch Actors using `.start()` (unbounded) or `.start_with_mailbox_capacity(n)` (bounded), with native back-pressure support.
*   **Extensible Launch Strategy**: The `Launch` trait allows for customizing the Actor's startup process (e.g., using different channel implementations or logging).

## Quick Start

Let's create a simple counter Actor.

**1. Add Dependencies:**

Add `acty` and `tokio` to your `Cargo.toml`.

```toml
[dependencies]
acty = "1"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

**2. Define the Actor:**

An Actor is simply a struct that implements the `acty::Actor` trait.

```rust
use acty::{Actor, ActorExt, AsyncClose};
use futures::{Stream, StreamExt};
use std::pin::pin;
use tokio::sync::oneshot;

// The Actor struct is typically empty or contains only initial configuration.
struct Counter;

// The message types handled by the Actor.
enum CounterMessage {
    Increment,
    GetValue(oneshot::Sender<u64>),
}

// Implement the Actor trait to define its core logic.
impl Actor for Counter {
    type Message = CounterMessage;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        let mut inbox = pin!(inbox);

        // The Actor's state is managed inside the run method.
        let mut value = 0;

        // Asynchronously process every incoming message.
        while let Some(msg) = inbox.next().await {
            match msg {
                CounterMessage::Increment => value += 1,
                CounterMessage::GetValue(responder) => {
                    // Send the result back via a oneshot channel.
                    let _ = responder.send(value);
                }
            }
        }
    }
}
```

**3. Launch and Interact:**

```rust
#[tokio::main]
async fn main() {
    // Use .start() to launch the Actor and get an Outbox handle.
    let counter = Counter.start();

    // Sending messages is asynchronous, but is immediate for unbounded channels.
    counter.send(CounterMessage::Increment).unwrap();
    counter.send(CounterMessage::Increment).unwrap();

    // Create a oneshot channel to retrieve the result.
    let (tx, rx) = oneshot::channel();
    counter.send(CounterMessage::GetValue(tx)).unwrap();

    // Wait for the result.
    let final_count = rx.await.expect("Actor did not respond");
    println!("Final count: {}", final_count);
    assert_eq!(final_count, 2);

    // Close the Outbox. Since this is the last Outbox, the Actor will automatically shut down.
    // .close().await waits for the Actor task to fully complete.
    counter.close().await;
}
```

## More Examples

Want to explore more complex patterns? Check out the `examples/` directory:

*   [`echo.rs`](./examples/echo.rs): The simplest "Hello World" Actor.
*   [`counter.rs`](./examples/counter.rs): Demonstrates state management and the request-response pattern.
*   [`manager_worker.rs`](./examples/manager_worker.rs): The classic manager-worker pattern, showing how an Actor can create and manage sub-Actors.
*   [`simulated_io.rs`](./examples/simulated_io.rs): Demonstrates how an Actor can perform asynchronous I/O operations while processing messages.
*   [`pub_sub.rs`](./examples/pub_sub.rs): The Publish-Subscribe pattern, demonstrating one-to-many message broadcasting.

## Contribution

Contributions in any form are welcome! Whether it's submitting issues, creating Pull Requests, or improving documentation, we appreciate your help.

## License

This project is distributed under either the MIT license or the Apache 2.0 license, at your option:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)
