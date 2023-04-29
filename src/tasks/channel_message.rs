use std::collections::HashMap;

use opentelemetry::{
    propagation::{Extractor, Injector, TextMapPropagator},
    trace::TraceContextExt,
};
use tracing_opentelemetry::OpenTelemetrySpanExt;

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

    pub fn link_to_span(&self, span: &tracing::Span) {
        let cx = self.extract();
        span.add_link(cx.span().span_context().clone());
    }

    pub fn inject(&mut self, cx: &opentelemetry::Context) {
        let propagator = opentelemetry::sdk::propagation::TraceContextPropagator::new();
        propagator.inject_context(cx, self);
    }

    pub fn extract(&self) -> opentelemetry::Context {
        let propagator = opentelemetry::sdk::propagation::TraceContextPropagator::new();
        propagator.extract(self)
    }
}

impl<T> Injector for ChannelMessage<T> {
    fn set(&mut self, key: &str, value: String) {
        self.context.insert(key.to_owned(), value);
    }
}

impl<T> Extractor for ChannelMessage<T> {
    fn get(&self, key: &str) -> Option<&str> {
        self.context.get(&key.to_owned()).map(|val| val.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.context.keys().map(|key| key.as_str()).collect()
    }
}
