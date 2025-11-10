use futures::Stream;

#[trait_variant::make(Send)]
pub trait Actor: Sized + 'static {
    type Message: Send;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send);
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{StreamExt, stream};
    use std::pin::pin;
    use std::sync::{Arc, Mutex};
    use tokio::sync::oneshot;

    /// 一个简单的 Actor，用于收集接收到的消息
    struct CollectorActor {
        // 使用 Arc<Mutex> 来安全地在 Actor 任务和测试任务之间共享结果
        results: Arc<Mutex<Vec<String>>>,
    }

    impl Actor for CollectorActor {
        type Message = String;

        async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
            let mut inbox = pin!(inbox);

            let mut collected_msgs = Vec::new();
            while let Some(msg) = inbox.next().await {
                collected_msgs.push(msg);
            }

            // 将收集到的结果写入共享状态
            let mut results = self.results.lock().unwrap();
            *results = collected_msgs;
        }
    }

    /// 一个用于测试返回结果的 Actor
    struct ResultActor {
        // 用于回传结果的 oneshot sender
        tx: oneshot::Sender<u64>,
    }

    impl Actor for ResultActor {
        type Message = u64;

        async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
            let mut inbox = Box::pin(inbox);

            let mut sum = 0;
            while let Some(val) = inbox.next().await {
                sum += val;
            }

            // 任务结束时发送最终结果
            self.tx.send(sum).unwrap_or(());
        }
    }

    #[tokio::test]
    async fn test_collector_actor_receives_all_messages() {
        let results_shared = Arc::new(Mutex::new(Vec::new()));

        let actor = CollectorActor {
            results: results_shared.clone(),
        };

        // 构造一个 Stream 作为 Inbox，模拟传入消息
        let messages = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let message_stream = stream::iter(messages);

        // 启动 Actor 任务
        let handle = tokio::spawn(actor.run(message_stream));

        // 等待 Actor 任务完成
        handle.await.expect("Actor task failed");

        // 验证 Actor 接收到的消息
        let received = results_shared.lock().unwrap();
        assert_eq!(received.len(), 3);
        assert_eq!(received[0], "A");
        assert_eq!(received[1], "B");
        assert_eq!(received[2], "C");
    }

    #[tokio::test]
    async fn test_result_actor_calculates_sum_and_returns_result() {
        let (tx, rx) = oneshot::channel();

        let actor = ResultActor { tx };

        // 构造输入流：1, 2, 3, 4
        let input = vec![1, 2, 3, 4];
        let message_stream = stream::iter(input);

        // 启动 Actor 任务
        let handle = tokio::spawn(actor.run(message_stream));

        // 等待任务完成
        handle.await.expect("Actor task failed");

        // 等待 Actor 回传的结果
        let sum = rx.await.expect("Failed to receive result from Actor");

        // 验证计算结果
        assert_eq!(sum, 1 + 2 + 3 + 4);
    }

    #[tokio::test]
    async fn test_actor_trait_bounds_and_lifecycle() {
        // 验证 Actor 即使没有消息也能正常启动和退出 (空 Stream)
        let (tx, rx) = oneshot::channel();
        let actor = ResultActor { tx };

        let empty_stream = stream::empty();

        let handle = tokio::spawn(actor.run(empty_stream));

        // 等待任务完成
        handle.await.expect("Actor task failed on empty stream");

        // 验证回传结果为初始值 0
        let sum = rx.await.expect("Failed to receive result");
        assert_eq!(sum, 0);
    }
}
