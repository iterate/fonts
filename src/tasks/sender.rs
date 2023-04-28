use async_channel::{SendError, Sender};

#[derive(Debug)]
pub struct ChannelMessage<T> {
    span_id: Option<tracing::Id>,
    body: T,
}

impl<T> ChannelMessage<T> {
    pub fn new(span_id: Option<tracing::Id>, body: T) -> Self {
        Self { body, span_id }
    }

    pub fn unwrap(&self) -> &T {
        &self.body
    }

    pub fn span_id(&self) -> Option<tracing::Id> {
        self.span_id.clone()
    }
}

pub async fn send_message_to_channel<T>(
    span_id: Option<tracing::Id>,
    channel: &Sender<ChannelMessage<T>>,
    message: T,
) -> Result<(), SendError<ChannelMessage<T>>> {
    let message = ChannelMessage::new(span_id, message);
    channel.send(message).await
}
