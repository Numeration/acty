use crate::Actor;

pub trait Launch<T>: Send + Sized + 'static {
    type Result;

    fn launch(self, actor: impl Actor<T>) -> Self::Result;
}