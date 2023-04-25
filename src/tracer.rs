use opentelemetry::{sdk::Resource, Key, KeyValue};
use tracing_subscriber::prelude::*;

pub fn init_tracing() -> eyre::Result<()> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .with_trace_config(
            opentelemetry::sdk::trace::config().with_resource(Resource::new(vec![KeyValue::new(
                Key::from_static_str("service.name"),
                "fonts",
            )])),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    let fmt_layer = tracing_subscriber::fmt::layer();
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer)
        .with(telemetry_layer)
        .init();

    Ok(())
}
