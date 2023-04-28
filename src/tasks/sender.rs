use async_channel::{SendError, Sender};

pub struct ChannelMessage<T> {
    body: T,
}

impl<T> ChannelMessage<T> {
    pub fn new(body: T) -> Self {
        Self { body }
    }

    pub fn unwrap(&self) -> &T {
        &self.body
    }
}

pub async fn send_message_to_channel<T>(
    channel: &Sender<ChannelMessage<T>>,
    message: T,
) -> Result<(), SendError<ChannelMessage<T>>> {
    let message = ChannelMessage::new(message);
    channel.send(message).await
}
