use futures::Stream;

#[trait_variant::make(Send)]
pub trait Actor: Sized + 'static {
    type Message: Send;

    async fn run(self, inbox: impl Stream<Item = Self::Message> + Send);
}
