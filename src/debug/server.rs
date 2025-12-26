use crate::domain::DebugEventReceiver;
use anyhow::Context;
use axum::Router;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use colored::Colorize;
use futures::stream::Stream;
use std::convert::Infallible;
use tokio::net::TcpListener;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::{Any, CorsLayer};

const EVENTS_PATH: &str = "/api/debug/events";
const ROOT_HTML: &str = include_str!("client/dist/index.html");
const DEPS_JS: &str = include_str!("client/dist/agx_debug.js");
const DEPS_CSS: &str = include_str!("client/dist/agx_debug.css");

pub struct DebugServer {
    debug_rx: DebugEventReceiver,
}

impl DebugServer {
    pub fn new(events_rx: DebugEventReceiver) -> Self {
        Self {
            debug_rx: events_rx,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let cors = CorsLayer::new().allow_methods(Any).allow_origin(Any);

        let app = Router::new()
            .route("/debug", get(root_get))
            .route("/agx_debug.js", get(js_get))
            .route("/agx_debug.css", get(css_get))
            .route(EVENTS_PATH, get(sse_handler))
            .with_state(self.debug_rx.clone())
            .layer(cors);

        let addr = "127.0.0.1:4880";

        let listener = TcpListener::bind(&addr)
            .await
            .with_context(|| format!(r#"couldn't bind TCP listener to address "{addr}""#))?;

        println!(
            "debug UI available at {}",
            format!("http://{}/debug", addr).green(),
        );
        axum::serve(listener, app)
            .await
            .context("couldn't start debug web server")?;

        Ok(())
    }
}

async fn root_get() -> impl IntoResponse {
    Html(ROOT_HTML.to_string())
}

async fn js_get() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    #[allow(clippy::expect_used)]
    headers.insert(
        "Content-Type",
        "text/javascript"
            .parse()
            .expect("content-type header value should've been parsed"),
    );

    (headers, DEPS_JS.to_string())
}

async fn css_get() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    #[allow(clippy::expect_used)]
    headers.insert(
        "Content-Type",
        "text/css"
            .parse()
            .expect("content-type header value should've been parsed"),
    );

    (headers, DEPS_CSS.to_string())
}

async fn sse_handler(
    State(debug_rx): State<DebugEventReceiver>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = debug_rx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let json = serde_json::to_string(&event).ok()?;
            Some(Ok(Event::default().data(json)))
        }
        Err(_) => None, // TODO: handle this error
    });

    Sse::new(stream)
}
