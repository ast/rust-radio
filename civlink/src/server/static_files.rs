// Copyright SM6WJM 2026

use axum::body::Body;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend/dist"]
struct Assets;

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        _ => "application/octet-stream",
    }
}

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try exact path, then fall back to index.html (SPA routing)
    let (file, effective_path) = if path.is_empty() {
        (Assets::get("index.html"), "index.html")
    } else {
        match Assets::get(path) {
            Some(f) => (Some(f), path),
            None => (Assets::get("index.html"), "index.html"),
        }
    };

    match file {
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, mime_for(effective_path))
            .body(Body::from(content.data.to_vec()))
            .expect("static-file response builder rejected a valid content-type header"),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
