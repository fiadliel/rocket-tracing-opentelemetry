use std::str::FromStr;

use http::{header::HeaderName, HeaderValue};
use opentelemetry::{propagation::TextMapPropagator, Context};
use opentelemetry_http::HeaderExtractor;
use rocket::{
    fairing,
    http::Status,
    request::{self, Outcome},
    Data, Request, Response,
};
use tracing::*;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug)]
pub struct TraceContext<'r> {
    span: &'r tracing::Span,
}

impl TraceContext<'_> {
    fn new(span: &tracing::Span) -> TraceContext {
        TraceContext { span }
    }

    fn from_request<'r>(request: &'r Request<'_>) -> Option<TraceContext<'r>> {
        let span = request.local_cache(|| None::<tracing::Span>);
        span.as_ref().map(TraceContext::new)
    }

    pub fn span(&self) -> &tracing::Span {
        self.span
    }
}

#[rocket::async_trait]
impl<'r> request::FromRequest<'r> for TraceContext<'r> {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match TraceContext::from_request(req) {
            Some(tc) => Outcome::Success(tc),
            None => Outcome::Failure((Status::InternalServerError, ())),
        }
    }
}

fn add_header<'r>(
    old_headers: &'r rocket::http::HeaderMap,
    new_headers: &'r mut http::HeaderMap,
    header_name: HeaderName,
) {
    if let Some(hn) = old_headers.get_one(header_name.as_str()) {
        if let Ok(hv) = HeaderValue::from_str(hn) {
            new_headers.append(header_name, hv);
        }
    }
}

pub struct TracingFairing {
    text_map_propagator: Box<dyn TextMapPropagator + Send + Sync>,
}

impl TracingFairing {
    pub fn new(text_map_propagator: Box<dyn TextMapPropagator + Send + Sync>) -> TracingFairing {
        TracingFairing {
            text_map_propagator,
        }
    }
}

#[rocket::async_trait]
impl fairing::Fairing for TracingFairing {
    fn info(&self) -> fairing::Info {
        fairing::Info {
            name: "Set up OpenTelemetry context",
            kind: fairing::Kind::Request | fairing::Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        let headers = request.headers();
        let mut carrier = http::HeaderMap::new();

        for f in self.text_map_propagator.fields() {
            let _ = HeaderName::from_str(f).map(|hn| {
                add_header(headers, &mut carrier, hn);
            });
        }

        let extractor = HeaderExtractor(&carrier);

        let context: Context = self.text_map_propagator.extract(&extractor);

        let span: tracing::Span = info_span!("http_request",
          otel.name = %format!("{} {}", request.method(), request.uri().path()),
          "http.target" = %request.uri().to_string(),
          "http.path" = %request.uri().path(),
          "http.method" = %request.method(),
          "http.status_code" = tracing::field::Empty
        );

        span.set_parent(context);
        request.local_cache(|| Some(span));
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if let Some(span) = request.local_cache(|| None::<tracing::Span>) {
            span.record("http.status_code", &response.status().code);
        }
    }
}
