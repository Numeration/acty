//! Counter Actor 示例
//!
//! 该示例展示了一个简单的计数器 Actor，演示以下功能：
//! - Actor 状态管理
//! - 异步处理消息
//! - 指令的同步回传（oneshot channel）
//! - 启动和结束 Actor
use acty::{Actor, ActorExt, Inbox};
use futures::StreamExt;
use std::pin::pin;

/// 计数器 Actor
///
/// 该 Actor 内部维护计数值 `value`，但结构体本身不包含字段。
/// 状态在 `run` 方法中异步维护。
struct Counter;

/// Counter Actor 的消息类型
///
/// Actor 通过接收这些消息来更新或提供计数值。
enum CounterMessage {
    /// 增加计数值
    Increment,

    /// 请求获取当前计数值
    ///
    /// 发送方通过 `tokio::sync::oneshot::Receiver<u64>` 获取计数结果
    GetValue(tokio::sync::oneshot::Sender<u64>),
}

/// 为 `Counter` Actor 实现 `Actor` trait
///
/// Actor 消息类型为 `CounterMessage`，内部异步维护计数值 `value`。
///
/// - 遍历 inbox 处理消息
/// - 根据消息更新状态或通过 oneshot 返回结果
impl Actor for Counter {
    type Message = CounterMessage;

    async fn run(self, inbox: impl Inbox<Item = Self::Message>) {
        // 将 inbox 固定在栈上以便异步迭代
        let mut inbox = pin!(inbox);

        // 计数值
        let mut value = 0;

        // 异步处理每条消息
        while let Some(msg) = inbox.next().await {
            match msg {
                // 增加计数
                CounterMessage::Increment => value += 1,

                // 通过 oneshot 返回当前计数
                // 如果发送失败（例如 Actor 已结束），则忽略错误
                CounterMessage::GetValue(tx) => tx.send(value).unwrap_or(()),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // 启动计数器 Actor
    let counter = Counter.start();

    // 发送三次自增指令
    counter.send(CounterMessage::Increment).unwrap_or(());
    counter.send(CounterMessage::Increment).unwrap_or(());
    counter.send(CounterMessage::Increment).unwrap_or(());

    // 创建 oneshot 通道以获取当前计数值
    let (rx, tx) = tokio::sync::oneshot::channel();
    counter.send(CounterMessage::GetValue(rx)).unwrap_or(());

    // 异步等待结果并打印
    println!("count: {}", tx.await.unwrap());

    // 解除 outbox 与 Actor 的绑定，确保 Actor 结束
    counter.detach().await;
}
