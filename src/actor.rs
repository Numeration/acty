use crate::Inbox;

#[trait_variant::make(Send)]
pub trait Actor<T>: Sized + 'static {

    async fn run(self, inbox: impl Inbox<Item = T>);
}
