use crate::launch::Launch;
use crate::{Actor, BoundedOutbox, UnboundedOutbox};
use futures::Stream;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};

pub trait ActorExt: Actor {
    fn with<L>(self, launch: L) -> L::Result<Self>
    where
        L: Launch<Message = Self::Message>,
    {
        launch.launch(self)
    }

    fn start_with<I>(self, inbox: I) -> JoinHandle<()>
    where
        I: Stream<Item = Self::Message> + Send + 'static,
    {
        tokio::spawn(self.run(inbox))
    }

    fn start_with_mailbox_capacity(self, mailbox_capacity: usize) -> BoundedOutbox<Self::Message> {
        let (sender, receiver) = tokio::sync::mpsc::channel(mailbox_capacity);
        let receiver_stream = ReceiverStream::new(receiver);
        BoundedOutbox::new(sender, self.start_with(receiver_stream))
    }

    fn start(self) -> UnboundedOutbox<Self::Message> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let receiver_stream = UnboundedReceiverStream::new(receiver);
        UnboundedOutbox::new(sender, self.start_with(receiver_stream))
    }
}

impl<A: Actor> ActorExt for A {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Actor;
    use futures::StreamExt;
    use std::pin::pin;
    use tokio::sync::oneshot;

    /// 一个简单的 Actor，用于收集消息并回传总数
    struct CountActor {
        // 用于回传最终计数的 sender
        result_tx: oneshot::Sender<u64>,
    }

    impl Actor for CountActor {
        type Message = String;

        async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
            let mut inbox = pin!(inbox);
            let mut count = 0;

            // 接收并消费所有消息
            while (inbox.next().await).is_some() {
                count += 1;
            }

            // 返回总数
            self.result_tx.send(count).unwrap_or(());
        }
    }

    /// 创建一个 CountActor 实例并返回其 oneshot 接收器
    fn create_count_actor() -> (CountActor, oneshot::Receiver<u64>) {
        let (tx, rx) = oneshot::channel();
        (CountActor { result_tx: tx }, rx)
    }

    #[tokio::test]
    async fn test_unbounded_start() {
        let (actor, rx) = create_count_actor();

        // 使用 start() 启动，返回 UnboundedOutbox
        let outbox = actor.start();

        // Unbounded Sender 是同步的
        outbox.send("msg1".to_string()).expect("Failed to send 1");
        outbox.send("msg2".to_string()).expect("Failed to send 2");

        // 关闭 Outbox，触发 Actor 退出
        outbox.close().await;

        // 验证 Actor 接收到的消息数量
        let count = rx.await.expect("Actor did not return result");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_bounded_start() {
        let (actor, rx) = create_count_actor();

        // 使用 capacity=5 启动 Bounded Outbox
        let outbox = actor.start_with_mailbox_capacity(5);

        for i in 0..3 {
            outbox
                .send(format!("msg{}", i))
                .await
                .expect("Failed to send");
        }

        // 关闭 Outbox，触发 Actor 退出
        outbox.close().await;

        // 验证 Actor 接收到的消息数量
        let count = rx.await.expect("Actor did not return result");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_bounded_start_with_backpressure() {
        // 使用容量为 1 的通道，并且 Actor 永远不消费消息，以便触发容量限制

        struct BlockingActor {
            start_tx: oneshot::Sender<()>,
            _stop_rx: oneshot::Receiver<()>,
        }

        impl Actor for BlockingActor {
            type Message = String;

            async fn run(self, _: impl Stream<Item = Self::Message> + Send) {
                // Actor 启动后立刻发送信号，然后阻塞，不消费 inbox
                self.start_tx.send(()).unwrap();
                // 任务不会退出，除非 Sender 释放
                self._stop_rx.await.unwrap_or(());
            }
        }

        let (start_tx, start_rx) = oneshot::channel();
        let (stop_tx, stop_rx) = oneshot::channel(); // 用于辅助清理

        let actor = BlockingActor {
            start_tx,
            _stop_rx: stop_rx,
        };

        let outbox = actor.start_with_mailbox_capacity(1);

        // 1. 等待 Actor 启动，确保通道已准备好接收
        start_rx.await.unwrap();

        // 2. 发送第一条消息 (成功，占用容量 1)
        outbox.send("m1".to_string()).await.unwrap();

        // 3. 尝试发送第二条消息，应该阻塞或等待，因为它满了。
        // 由于 tokio::mpsc::Sender::send() 会等待空间，我们使用 `timeout` 来验证它会阻塞。
        let send_future = outbox.send("m2".to_string());

        let timeout_result =
            tokio::time::timeout(tokio::time::Duration::from_millis(50), send_future).await;

        // 验证发送超时，意味着它被阻塞了（背压生效）
        assert!(
            timeout_result.is_err(),
            "Send should have timed out due to full channel"
        );

        // 4. 清理：释放 Outbox，停止 BlockingActor
        drop(stop_tx); // 释放 Actor 辅助资源
        outbox.close().await;
    }
}
