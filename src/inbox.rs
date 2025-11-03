use futures::Stream;

pub trait Inbox: Stream + Send {}

impl<S: Stream + Send> Inbox for S {}
