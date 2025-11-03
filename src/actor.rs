use crate::Inbox;

#[trait_variant::make(Send)]
pub trait Actor: Sized + 'static {
    type Message<'msg>: Send + 'static;

    async fn run<'msg>(self, inbox: impl Inbox<Item = Self::Message<'msg>>);
}
