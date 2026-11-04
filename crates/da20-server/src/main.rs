use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(RustEmbed)]
#[folder = "../../dist/"]
struct Assets;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/data/performance.json", get(serve_performance_placeholder))
        .fallback(static_handler)
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("public, max-age=300"),
        ));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

async fn serve_performance_placeholder() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        include_str!("../../../data/TODO_performance_data.json"),
    )
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => {
            // SPA fallback
            if let Some(index) = Assets::get("index.html") {
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(Body::from(index.data.into_owned()))
                    .unwrap()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    "UI assets missing — build wasm to dist/ first",
                )
                    .into_response()
            }
        }
    }
}
