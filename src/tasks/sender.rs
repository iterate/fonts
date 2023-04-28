use std::collections::HashMap;

use async_channel::{SendError, Sender};
use opentelemetry::propagation::Injector;

#[derive(Debug)]
pub struct ChannelMessage<T> {
    context: HashMap<String, String>,
    body: T,
}

impl<T> ChannelMessage<T> {
    pub fn new(body: T) -> Self {
        Self {
            context: Default::default(),
            body,
        }
    }

    pub fn unwrap(&self) -> &T {
        &self.body
    }
}

impl<T> Injector for ChannelMessage<T> {
    fn set(&mut self, key: &str, value: String) {
        self.context.insert(key.to_owned(), value);
    }
}
