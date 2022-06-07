# rocket-tracing-opentelemetry

Hooks up Rocket with OpenTelemetry context (via tracing). Includes ability to capture OT context from Rocket request headers.

Use like this:

```rust
#[macro_use]
extern crate rocket;

use rocket_tracing_opentelemetry::TracingFairing;

#[launch]
async fn rocket() -> _ {
    let tracing_fairing = TracingFairing::new(Box::new(TraceContextPropagator::new()));
    let tracer_provider = todo!("Must create and register a tracer provider!");

    rocket::build()
        .manage(tracer_provider)
        .mount("/", routes())
        .attach(tracing_fairing)
}
```

```rust
#[get("/hello/<name>/<age>")]
#[instrument(parent = trace_ctx.span(), skip(trace_ctx))]
fn hello(trace_ctx: TraceContext<'_>, name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}
```
