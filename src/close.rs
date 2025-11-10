
#[trait_variant::make(Send)]
pub trait AsyncClose {
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