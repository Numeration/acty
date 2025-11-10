//! Provides a unified asynchronous closing interface: [`AsyncClose`].
//!
//! The main purpose of this trait is to abstract the closing behavior of different
//! types of `Outbox` (e.g., `Outbox` wrapping `mpsc::Sender` or `mpsc::UnboundedSender`).
//! By implementing this trait, we provide a consistent `.close().await` method for all
//! `Outbox` types, which is used to gracefully close the connection to the Actor
//! and wait for its task to terminate.

#[trait_variant::make(Send)]
pub trait AsyncClose {
    /// Asynchronously closes the associated resource.
    ///
    /// For senders, this typically means dropping the sender to signal the end
    /// of the message stream.
    async fn close(self);
}

impl<T: Send> AsyncClose for tokio::sync::mpsc::Sender<T> {
    #[inline]
    async fn close(self) {
        drop(self);
    }
}

impl<T: Send> AsyncClose for tokio::sync::mpsc::UnboundedSender<T> {
    #[inline]
    async fn close(self) {
        drop(self);
    }
}
