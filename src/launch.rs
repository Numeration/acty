use crate::Actor;

pub trait Launch: Send + Sized {
    type Message<'msg>: Send + 'static;
    type Result<A>;

    fn launch<A: for<'msg> Actor<Message<'msg> = Self::Message<'msg>>>(
        self,
        actor: A,
    ) -> Self::Result<A>;
}
