//! Manager-Worker Pattern Example
//!
//! This example demonstrates a common concurrency pattern in the Actor model:
//! - A `ManagerActor` receives a complex computation task.
//! - The `ManagerActor` breaks the task down into smaller chunks.
//! - It launches a `WorkerActor` for each small chunk of the task.
//! - The `WorkerActor` completes the computation and reports the result back to the `ManagerActor`.
//! - The `ManagerActor` collects all results, aggregates them, and returns the final sum to the original caller.
//!
//! This illustrates:
//! - How Actors can create and manage other Actors (sub-Actors).
//! - Request-response communication between Actors.
//! - Using `futures::future::join_all` to elegantly wait for multiple parallel Actor tasks.

use acty::{Actor, ActorExt, AsyncClose};
use futures::{Stream, StreamExt, future::join_all};
use std::pin::pin;
use tokio::sync::oneshot;

/// Worker Actor, responsible for executing a small chunk of a computation task.
struct Worker;

/// Message for the Worker Actor.
/// It receives a task range (start, end) and a `oneshot::Sender` to return the result.
struct ComputeChunk {
    start: u64,
    end: u64,
    responder: oneshot::Sender<u64>,
}

impl Actor for Worker {
    type Message = ComputeChunk;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        let mut inbox = pin!(inbox);

        // A Worker typically handles one task and then finishes.
        if let Some(msg) = inbox.next().await {
            let sum = (msg.start..=msg.end).sum();
            // Send the computation result back
            msg.responder.send(sum).unwrap_or(());
        }
    }
}

/// Manager Actor, responsible for decomposing tasks and distributing them to Workers.
struct Manager {
    num_workers: u64,
}

/// Message for the Manager Actor.
/// It receives a large computation task and needs to return the final result.
struct ComputeSum {
    start: u64,
    end: u64,
    responder: oneshot::Sender<u64>,
}

impl Actor for Manager {
    type Message = ComputeSum;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        let mut inbox = pin!(inbox);

        while let Some(msg) = inbox.next().await {
            println!(
                "Manager: Received job to sum from {} to {}.",
                msg.start, msg.end
            );

            let total_range = msg.end - msg.start + 1;
            let chunk_size = total_range.div_ceil(self.num_workers);

            let mut response_rxs = Vec::new();

            for i in 0..self.num_workers {
                let chunk_start = msg.start + i * chunk_size;
                let chunk_end = (chunk_start + chunk_size - 1).min(msg.end);

                if chunk_start > msg.end {
                    break;
                }

                println!(
                    "Manager: Spawning worker for range {}..={}",
                    chunk_start, chunk_end
                );

                // Create a oneshot channel for each task
                let (worker_tx, worker_rx) = oneshot::channel();
                response_rxs.push(worker_rx);

                // Launch the Worker Actor
                let worker = Worker.start();
                // Send the task to the Worker
                let worker_msg = ComputeChunk {
                    start: chunk_start,
                    end: chunk_end,
                    responder: worker_tx,
                };
                worker.send(worker_msg).unwrap_or(());

                // We don't need to hold the worker's outbox. Since the outbox is dropped
                // immediately after sending the message, the worker will automatically
                // terminate after processing that single message (sender-driven lifecycle).
            }

            // Await the results from all workers in parallel
            let worker_results = join_all(response_rxs).await;

            // Aggregate the results
            let total_sum: u64 = worker_results.into_iter().map(|res| res.unwrap_or(0)).sum();

            // Send the final result back
            msg.responder.send(total_sum).unwrap_or(());
        }
    }
}

#[tokio::main]
async fn main() {
    // Launch the Manager Actor
    let manager = Manager { num_workers: 4 }.start();

    // Prepare the task
    let (tx, rx) = oneshot::channel();
    let job = ComputeSum {
        start: 1,
        end: 100,
        responder: tx,
    };

    // Send the task to the Manager
    manager.send(job).unwrap_or(());

    // Wait for the final result
    let result = rx.await.expect("Manager did not respond");
    let expected: u64 = (1..=100).sum();

    println!("\nFinal result: {}, Expected: {}", result, expected);
    assert_eq!(result, expected);

    // Close the Manager Actor
    manager.close().await;
}
