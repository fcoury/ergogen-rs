use axum::{
    body::Body,
    extract::DefaultBodyLimit,
    handler::HandlerWithoutStateExt as _,
    http::{header, StatusCode},
    response::Response,
    routing::get_service,
    Router,
};
use tokio::net::TcpListener;
use tower_http::{services::ServeDir, trace::TraceLayer};

const MAX_BODY_SIZE: usize = 50 * 1024 * 1024; // 50 MB

pub async fn start(listen_addr: Option<String>, port: Option<u16>) -> anyhow::Result<()> {
    let listen_addr = listen_addr.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = port.unwrap_or(3000);

    tracing::info!("Starting server on {}:{}", listen_addr, port);

    let static_files =
        get_service(ServeDir::new("static").not_found_service(spa_fallback.into_service()));

    let router = Router::new()
        // .nest("/api", api_router)
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE));

    let listener = TcpListener::bind(format!("{}:{}", listen_addr, port)).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

async fn spa_fallback() -> Response {
    match tokio::fs::read("static/index.html").await {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(content))
            .unwrap(), // Safe since we are correctly constructing a response
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("index.html not found"))
            .unwrap(),
    }
}
