//! Echo Actor 示例
//!
//! 该示例展示了如何实现一个最简单的 Actor，并演示以下功能：
//! - 实现 Actor trait
//! - 处理 Inbox 消息
//! - 发送消息到 Actor
//! - 正确释放 Outbox
use std::pin::pin;
use futures::StreamExt;
use acty::{Actor, Inbox, ActorExt};

/// 一个最简单的 Echo Actor
///
/// 该 Actor 处理类型为 `String` 的消息，并将其打印到标准输出。
/// 注意：该 Actor 没有状态字段，仅展示最基本的用法。
struct Echo;

/// 实现 `Actor` trait，消息类型为 `String`
///
/// 当 Inbox 收到消息时，会逐条打印到标准输出。
impl Actor<String> for Echo {
    async fn run(self, inbox: impl Inbox<Item = String>) {
        // 将 inbox 固定在栈上，以便异步迭代。
        // 如果需要跨函数传递，可使用 Box::pin 将其固定到堆上（带一点分配开销）。
        let mut inbox = pin!(inbox);

        // 异步遍历 inbox 中的每条消息
        while let Some(msg) = inbox.next().await {
            println!("echo: {}", msg);
        }
    }
}

#[tokio::main]
async fn main() {
    // 启动 actor 并获得与之绑定的 outbox
    let echo = Echo.start();

    // 向 Actor 发送消息
    // send 返回 Result，当 Actor 已关闭时会返回 Err
    // 这里使用 unwrap_or(()) 忽略可能的错误
    echo.send("Hello".to_string()).unwrap_or(());
    echo.send("World".to_string()).unwrap_or(());
    echo.send("!".to_string()).unwrap_or(());

    // 解除 outbox 与 actor 的绑定，
    // 确保 actor 收到所有消息后自然结束
    echo.detach().await;
}
