use crate::Actor;

pub trait Launch: Send + Sized {
    type Message: Send;

    type Result<A>;

    fn launch<A>(self, actor: A) -> Self::Result<A>
    where
        A: Actor<Message = Self::Message>;
}