use crate::Inbox;

#[trait_variant::make(Send)]
pub trait Actor: Sized + 'static {

    type Message: Send;

    async fn run(self, inbox: impl Inbox<Item = Self::Message>);
}
