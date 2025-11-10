use crate::close::AsyncClose;
use std::ops::Deref;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct ActorHandle {
    join_handle: Arc<JoinHandle<()>>,
}

impl ActorHandle {
    fn new(join_handle: JoinHandle<()>) -> Self {
        Self {
            join_handle: Arc::new(join_handle),
        }
    }

    async fn wait_for_completion(mut self) {
        if let Some(join_handle) = Arc::get_mut(&mut self.join_handle) {
            join_handle.await.unwrap();
        }
    }
}

#[derive(Debug, Clone)]
pub struct Outbox<S> {
    sender: S,
    handle: ActorHandle,
}

impl<S> Outbox<S> {
    pub fn new(sender: S, join_handle: JoinHandle<()>) -> Self {
        Self {
            sender,
            handle: ActorHandle::new(join_handle),
        }
    }
}

impl<S: AsyncClose> Outbox<S> {
    pub async fn close(self) {
        self.sender.close().await;
        self.handle.wait_for_completion().await;
    }
}

impl<S> Deref for Outbox<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}

pub type BoundedOutbox<T> = Outbox<tokio::sync::mpsc::Sender<T>>;

pub type UnboundedOutbox<T> = Outbox<tokio::sync::mpsc::UnboundedSender<T>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Actor;
    use futures::{Stream, StreamExt};
    use tokio::sync::{mpsc, oneshot};
    use tokio_stream::wrappers::UnboundedReceiverStream;

    // Actor 的 Run 逻辑：等待 start_signal 确认启动，等待 exit_signal 确认退出
    // 这允许我们精确掌握 Actor 的生命周期，用于测试 Outbox 引用计数。
    struct LifecycleActor {
        // Actor 启动信号
        start_tx: oneshot::Sender<()>,

        // Actor 退出信号
        exit_tx: oneshot::Sender<()>,
    }

    impl Actor for LifecycleActor {
        type Message = u32;

        async fn run(self, inbox: impl Stream<Item = Self::Message> + Send) {
            // 立即发送启动信号
            self.start_tx.send(()).unwrap();

            // 耗尽 inbox
            let _ = inbox.for_each(|_| async {}).await;

            // 最后发送退出信号
            self.exit_tx.send(()).unwrap_or(());
        }
    }

    // 启动一个受控的 LifecycleActor
    fn launch_controlled_actor() -> (
        Outbox<mpsc::UnboundedSender<u32>>,
        oneshot::Receiver<()>,
        oneshot::Receiver<()>,
    ) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (start_tx, start_rx) = oneshot::channel();
        let (exit_tx, exit_rx) = oneshot::channel();

        let actor = LifecycleActor { start_tx, exit_tx };
        let join_handle = tokio::spawn(actor.run(UnboundedReceiverStream::new(receiver)));

        let outbox = Outbox::new(sender, join_handle);

        (outbox, start_rx, exit_rx)
    }

    #[tokio::test]
    async fn test_outbox_deref_to_sender() {
        let (outbox, start_rx, _) = launch_controlled_actor();

        // 等待 Actor 启动
        start_rx.await.unwrap();

        // 验证 Deref 实现，可以直接调用 send
        outbox.send(42).unwrap();

        // 清理
        outbox.close().await;
    }

    #[tokio::test]
    async fn test_single_outbox_waits_for_completion() {
        let (outbox, start_rx, exit_rx) = launch_controlled_actor();

        // 等待 Actor 启动
        start_rx.await.unwrap();

        // 调用 close().await，预期会等待任务完成
        outbox.close().await;

        // 检查 Actor 是否退出
        assert!(exit_rx.await.is_ok());
    }

    #[tokio::test]
    async fn test_cloned_outbox_only_last_one_waits() {
        let (outbox, start_rx, mut exit_rx) = launch_controlled_actor();

        // 等待 Actor 启动
        start_rx.await.unwrap();

        // 1. 克隆 Outbox
        let clone1 = outbox.clone();
        let clone2 = outbox;

        // ---- 第一次关闭：非最后一个引用 ----

        // clone1 调用 close()。由于 clone2 仍然持有 ActorHandle，它不应该等待
        clone1.close().await;

        // clone1 调用 close 并不会导致 actor 退出
        assert!(exit_rx.try_recv().is_err());

        // ---- 第二次关闭：最后一个引用 ----

        // clone2 调用 close()。它是最后一个 Arc 引用，它应该等待 JoinHandle
        clone2.close().await;

        // 验证第二次关闭是否会让 Actor 退出
        assert!(
            exit_rx.await.is_ok(),
            "Second close should not wait indefinitely if Actor is already finished."
        );
    }

    #[tokio::test]
    async fn test_outbox_release_closes_channel() {
        let (sender, mut receiver) = mpsc::unbounded_channel::<i32>();

        // 创建一个空的 JoinHandle，用于模拟 Actor 任务（不重要，只需要结构）
        let handle = tokio::spawn(async {});
        let outbox = Outbox::new(sender, handle);

        // 发送消息
        outbox.send(1).unwrap();

        // 释放 Outbox
        outbox.close().await;

        // 验证 Receiver 还能收到已发送的消息
        assert_eq!(receiver.recv().await, Some(1));

        // 验证 Receiver 收到 None，表示 Sender 已全部关闭
        assert_eq!(receiver.recv().await, None);
    }
}
