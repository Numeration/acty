use futures::Stream;

pub trait Inbox: Stream + Send + 'static {}

impl<S: Stream + Send + 'static> Inbox for S {}
