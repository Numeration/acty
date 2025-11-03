use crate::Actor;

pub trait Launch: Send + Sized {
    type Message: Send;
    type Result<A>;

    fn launch<A: Actor<Message = Self::Message>>(self, actor: A) -> Self::Result<A>;
}
