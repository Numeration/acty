//! # Acty
//!
//! Acty 旨在基于 tokio 实现一个高性能且非常轻量的 actor 框架。
//!
//! ## 核心概念
//!
//! - **Actor**: 只需实现 [Actor] trait 即可。
//! - **Start**: 所有 Actor 自动实现 [ActorExt]，可用于快速启动。
//! - **Inbox / Outbox**: Actor 通过 [inbox] 接收消息，[outbox] 用于发送消息。
//! - **生命周期**: 发送方负责制，Actor 会在无法再接收消息时自然结束。
//!
//! ## 协作式取消
//!
//! 如果需要主动中断 Actor，可通过协作式取消通知发送方停止发送消息。
//!
//! ## 回传结果
//!
//! Actor 可以在结束时通过回传钩子发送结果。示例：
//! ```rust
//!
//! use std::pin::pin;
//! use std::fmt::Write;
//! use futures::StreamExt;
//! use acty::{Actor, ActorExt, Inbox};
//!
//! // 定义 actor 对象
//! struct MyActor {
//!    // 结果回传钩子
//!    result: tokio::sync::oneshot::Sender<String>,
//! }
//!
//! // 实现 actor 特征
//! impl Actor for MyActor {
//!     type Message = String;
//!
//!     async fn run(self, inbox: impl Inbox<Item = Self::Message>) {
//!         // 将 inbox 固定在当前栈上以便使用 StreamExt::next()
//!         // 详细请见 examples/echo.rs
//!         let mut inbox = pin!(inbox);
//!
//!         // 初始化此 actor 的状态
//!         let mut article = String::new();
//!         let mut count = 1;
//!
//!         // 处理 inbox 中的消息
//!         while let Some(msg) = inbox.next().await {
//!             write!(article, "part {}: {}\n", count, msg.as_str()).unwrap();
//!             count += 1;
//!         }
//!
//!         // 结束时将 article 通过回传钩子进行回传
//!         self.result.send(article);
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // 创建结果回传钩子
//!     let (tx, rx) = tokio::sync::oneshot::channel();
//!     // 初始化 actor
//!     let my_actor = MyActor { result: tx }.start();
//!
//!     // 向 actor 发送消息
//!     my_actor.send("It's a good day today.".to_string());
//!     my_actor.send("Let's have some tea first.".to_string());
//!     // 确保 outbox 被释放，触发 actor 结束
//!     my_actor.detach().await;
//!
//!     // 获得 actor 的运行结果
//!     assert_eq!(
//!         "part 1: It's a good day today.\npart 2: Let's have some tea first.\n",
//!         rx.await.expect("Actor did not send result")
//!     );
//! }
//! ```
//!
//! 更多示例请看 examples 目录。

mod actor;
mod inbox;
mod launch;
mod outbox;
mod start;
mod close;

pub use {
    actor::Actor, inbox::Inbox, launch::Launch, outbox::BoundedOutbox, outbox::UnboundedOutbox,
    start::ActorExt,
};
