use std::collections::HashMap;

use opentelemetry::{
    propagation::{Extractor, Injector, TextMapPropagator},
    trace::TraceContextExt,
};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug)]
pub struct ChannelMessage<T> {
    context: HashMap<String, String>,
    root_span: tracing::Span,
    body: T,
}

impl<T> ChannelMessage<T> {
    pub fn new(root_span: tracing::Span, body: T) -> Self {
        Self {
            context: Default::default(),
            root_span,
            body,
        }
    }

    pub fn unwrap(&self) -> &T {
        &self.body
    }

    pub fn root_span(&self) -> &tracing::Span {
        &self.root_span
    }

    #[allow(unused)]
    pub fn set_parent(&self, span: &tracing::Span) {
        let cx = self.extract();
        span.set_parent(cx);
    }

    #[allow(unused)]
    pub fn set_link(&self, span: &tracing::Span) {
        let cx = self.extract();
        span.add_link(cx.span().span_context().clone())
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
