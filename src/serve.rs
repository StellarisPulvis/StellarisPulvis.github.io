use anyhow::Result;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use std::{
    convert::Infallible,
    net::SocketAddr,
    path::Path,
    sync::Arc,
    time::Duration,
};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

pub async fn start_server(
    output_dir: &Path,
    port: u16,
    tx: broadcast::Sender<String>,
) -> Result<()> {
    let state = Arc::new(AppState { tx });

    let app = Router::new()
        .route("/__reload__", get(sse_handler))
        .fallback_service(
            ServeDir::new(output_dir)
                .append_index_html_on_directories(true),
        )
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("开发服务器已启动: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

struct AppState {
    tx: broadcast::Sender<String>,
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();

    let stream = futures::stream::unfold(rx, |mut rx| async {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    return Some((Ok(Event::default().data(msg)), rx));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}
