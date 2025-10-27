use crate::Actor;

pub trait Launch: Send + Sized + 'static {
    
    type Message;
    
    type Result;

    fn launch(self, actor: impl Actor<Message = Self::Message>) -> Self::Result;
}