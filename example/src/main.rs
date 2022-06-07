#[macro_use]
extern crate rocket;

use opentelemetry::{
    global,
    sdk::{export::trace::stdout, propagation::TraceContextPropagator},
};
use rocket::fairing::AdHoc;
use rocket_tracing_opentelemetry::*;
use tracing_attributes::instrument;
use tracing_subscriber::{subscribe::CollectExt, util::SubscriberInitExt, Registry};

#[get("/")]
#[instrument(parent = trace_ctx.span(), skip(trace_ctx))]
async fn index(trace_ctx: TraceContext<'_>) -> &str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    let tracer = stdout::new_pipeline()
        .with_pretty_print(true)
        .install_simple();

    let telemetry = tracing_opentelemetry::OpenTelemetrySubscriber::new(tracer);
    Registry::default().with(telemetry).init();

    let tracing_fairing = TracingFairing::new(Box::new(TraceContextPropagator::new()));

    rocket::build()
        .mount("/", routes![index])
        .attach(tracing_fairing)
        .attach(AdHoc::on_shutdown("Shutdown OT", |_| {
            Box::pin(async move {
                let _ = rocket::tokio::task::spawn_blocking(|| global::shutdown_tracer_provider())
                    .await;
            })
        }))
}
