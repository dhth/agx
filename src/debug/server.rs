use crate::domain::DebugEventReceiver;
use anyhow::Context;
use axum::Router;
use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use futures::stream::Stream;
use std::convert::Infallible;
use tokio::net::TcpListener;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::{Any, CorsLayer};

const EVENTS_PATH: &str = "/api/debug/events";

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
            .route(EVENTS_PATH, get(sse_handler))
            .with_state(self.debug_rx.clone())
            .layer(cors);

        let addr = "127.0.0.1:4880";

        let listener = TcpListener::bind(&addr)
            .await
            .with_context(|| format!(r#"couldn't bind TCP listener to address "{addr}""#))?;

        println!("starting debug server at http://{}", addr);
        axum::serve(listener, app)
            .await
            .context("couldn't start debug web server")?;

        Ok(())
    }
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
        Err(_) => None,
    });

    Sse::new(stream)
}
