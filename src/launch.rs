use crate::Actor;

pub trait Launch: Send + Sized + 'static {
    type Message;
    type Result<A>;

    fn launch<A: Actor<Message = Self::Message>>(self, actor: A) -> Self::Result<A>;
}
