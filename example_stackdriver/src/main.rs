#[macro_use]
extern crate rocket;

use opentelemetry::{
    global, runtime::Tokio, sdk::propagation::TraceContextPropagator, trace::TracerProvider,
};
use rocket::fairing::AdHoc;
use rocket_tracing_opentelemetry::*;
use tracing_subscriber::{filter, prelude::*, Registry};

#[get("/")]
#[tracing::instrument(parent = trace_ctx.span(), skip(trace_ctx))]
async fn index(trace_ctx: TraceContext<'_>) -> &str {
    "Hello, world!"
}

#[launch]
async fn rocket() -> _ {
    let authenticator = opentelemetry_stackdriver::GcpAuthorizer::new()
        .await
        .unwrap();

    let (exporter, driver) = opentelemetry_stackdriver::StackDriverExporter::builder()
        .build(authenticator)
        .await
        .unwrap();

    let tracer_provider = opentelemetry::sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, Tokio)
        .build();

    rocket::tokio::spawn(driver);

    // Filter out some non-request related tracing
    let telemetry = tracing_opentelemetry::OpenTelemetryLayer::new(tracer_provider.tracer(""))
        .with_filter(filter::FilterFn::new(|metadata| {
            !(metadata.target().starts_with("mio::")
                || metadata.target().starts_with("h2::")
                || metadata.target().starts_with("hyper::")
                || metadata.target().starts_with("tokio_util::"))
        }));

    Registry::default().with(telemetry).init();

    let tracing_fairing = TracingFairing::new(Box::new(TraceContextPropagator::new()), true);

    rocket::build()
        .mount("/", routes![index])
        .manage(tracer_provider)
        .attach(tracing_fairing)
        .attach(AdHoc::on_shutdown("Shutdown OpenTelemetry", |_| {
            Box::pin(async move {
                let _ = rocket::tokio::task::spawn_blocking(|| global::shutdown_tracer_provider());
            })
        }))
}
