#[macro_use]
extern crate rocket;

use opentelemetry::sdk::propagation::TraceContextPropagator;
use rocket_tracing_opentelemetry::*;
use tracing_subscriber::fmt::format::FmtSpan;

#[get("/")]
#[tracing::instrument(parent = trace_ctx.span(), skip(trace_ctx))]
async fn index(trace_ctx: TraceContext<'_>) -> &str {
    "Hello, world!"
}

#[launch]
async fn rocket() -> _ {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::ACTIVE)
        .init();

    let tracing_fairing = TracingFairing::new(Box::new(TraceContextPropagator::new()), false);

    rocket::build()
        .mount("/", routes![index])
        .attach(tracing_fairing)
}
