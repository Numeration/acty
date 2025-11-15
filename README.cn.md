# Acty: 一个为 Tokio 设计的轻量级 Actor 框架

[English](./README.md) | **简体中文**

[![Crates.io](https://img.shields.io/crates/v/acty.svg)](https://crates.io/crates/acty)
[![Docs.rs](https://docs.rs/acty/badge.svg)](https://docs.rs/acty)

**Acty** 是一个基于 [Tokio](https://tokio.rs/) 构建的、高性能且极其轻量的 Actor 框架。它旨在为 Rust 异步生态提供一个简单、安全且符合人体工程学的并发模型。

## 核心理念

Acty 的设计围绕着几个核心原则：

*   **极简主义**: 只需实现一个 `Actor` trait。没有复杂的生命周期钩子，没有宏魔法。
*   **发送方驱动的生命周期**: Actor 的生命周期由其消息发送方（`Outbox`）完全控制。当所有 `Outbox` 都被销毁后，Actor 会自动、优雅地关闭。这消除了手动管理 Actor 停机的需要，从根本上避免了资源泄漏。
*   **状态隔离**: Actor 的状态作为局部变量存在于其核心的 `run` 方法中，而不是结构体字段。这使得状态管理清晰、简单，并天然地避免了数据竞争。
*   **与 Tokio 生态无缝集成**: Acty 构建于 Tokio 的基础组件之上，如 `mpsc` 通道和异步 `Stream`，可以轻松地与 Tokio 生态中的任何库（如 `hyper`, `reqwest`, `tonic`）组合使用。

## 特性

*   **轻量级**: 核心 API 仅包含几个关键的 trait (`Actor`, `ActorExt`, `AsyncClose`)。
*   **类型安全**: 利用 Rust 的类型系统确保 Actor 消息传递的正确性。
*   **高性能**: 底层直接使用 Tokio 的 MPSC 通道，性能开销极低。
*   **支持有界和无界通道**: 通过 `.start()` (无界) 或 `.start_with_mailbox_capacity(n)` (有界) 轻松启动 Actor，并天然支持背压。
*   **可扩展的启动策略**: 通过 `Launch` trait，可以自定义 Actor 的启动过程（例如，使用不同的通道实现或日志记录）。

## 快速上手

让我们创建一个简单的计数器 Actor。

**1. 添加依赖:**

在你的 `Cargo.toml` 中添加 `acty` 和 `tokio`。

```toml
[dependencies]
acty = "1"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

**2. 定义 Actor:**

一个 Actor 就是一个实现了 `acty::Actor` trait 的结构体。

```rust
use acty::{Actor, ActorExt, AsyncClose};
use futures::{Stream, StreamExt};
use std::pin::pin;
use tokio::sync::oneshot;

// Actor 结构体通常是空的，或者只包含初始配置。
struct Counter;

// Actor 处理的消息类型。
enum CounterMessage {
    Increment,
    GetValue(oneshot::Sender<u64>),
}

// 实现 Actor trait 定义其核心逻辑。
impl Actor for Counter {
    type Message = CounterMessage;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
        let mut inbox = pin!(inbox);

        // Actor 的状态在 run 方法内部管理。
        let mut value = 0;

        // 异步处理收到的每一条消息。
        while let Some(msg) = inbox.next().await {
            match msg {
                CounterMessage::Increment => value += 1,
                CounterMessage::GetValue(responder) => {
                    // 通过 oneshot channel 将结果发送回去。
                    let _ = responder.send(value);
                }
            }
        }
    }
}
```

**3. 启动并与之交互:**

```rust
#[tokio::main]
async fn main() {
    // 使用 .start() 启动 Actor 并获取一个 Outbox 句柄。
    let counter = Counter.start();

    // 发送消息是异步的，但对于无界通道是立即返回的。
    counter.send(CounterMessage::Increment).unwrap();
    counter.send(CounterMessage::Increment).unwrap();

    // 创建一个 oneshot channel 来获取结果。
    let (tx, rx) = oneshot::channel();
    counter.send(CounterMessage::GetValue(tx)).unwrap();

    // 等待结果。
    let final_count = rx.await.expect("Actor did not respond");
    println!("Final count: {}", final_count);
    assert_eq!(final_count, 2);

    // 关闭 Outbox。由于这是最后一个 Outbox，Actor 会自动关闭。
    // .close().await 会等待 Actor 任务完全结束。
    counter.close().await;
}
```

## 更多示例

想要探索更复杂的模式吗？请查看 `examples/` 目录：

*   [`echo.rs`](./examples/echo.rs): 最简单的 "Hello World" Actor。
*   [`counter.rs`](./examples/counter.rs): 演示状态管理和请求-响应模式。
*   [`manager_worker.rs`](./examples/manager_worker.rs): 经典的管理器-工作者模式，演示 Actor 如何创建和管理子 Actor。
*   [`simulated_io.rs`](./examples/simulated_io.rs): 演示 Actor 如何在处理消息时执行异步 I/O 操作。
*   [`pub_sub.rs`](./examples/pub_sub.rs): 发布-订阅模式，演示一对多的消息广播。

## 贡献

欢迎任何形式的贡献！无论是提交 issue、发起 Pull Request 还是改进文档，我们都非常欢迎。

## 许可证

本项目采用 [Apache 2.0 许可证](LICENSE-APACHE)。

---
