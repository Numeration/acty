use crate::Actor;

pub trait Launch: Send + Sized {
    type Message<'msg>;
    type Result<A>;

    fn launch<'msg, A: Actor<Message<'msg> = Self::Message<'msg>>>(
        self,
        actor: A,
    ) -> Self::Result<A>;
}
