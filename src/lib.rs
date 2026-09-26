#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub mod bitwarden;
pub mod db;
pub mod error;
pub mod http;
pub mod mcp;
pub mod service;

use std::sync::Arc;

use axum::{
    Router,
    http::{Uri, header},
    response::{IntoResponse, Response},
    routing::{any, get, post},
};
use tower_http::trace::TraceLayer;

pub use db::Store;
pub use error::Error;

static UI: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/client/dist");

pub fn app(store: Store) -> Router {
    let store = Arc::new(store);
    let api = Router::new()
        .route("/health", get(http::api_health))
        .route("/services", get(http::list).post(http::create))
        .route(
            "/services/{id}",
            get(http::get).patch(http::update).delete(http::delete),
        )
        .route("/import/bitwarden", post(http::import_bitwarden))
        .fallback(http::api_not_found);

    Router::new()
        .route("/healthz", get(http::healthz))
        .route("/api", any(http::api_not_found))
        .route("/api/", any(http::api_not_found))
        .nest("/api", api)
        .merge(mcp::endpoint(&store))
        .fallback_service(get(ui))
        .layer(TraceLayer::new_for_http())
        .with_state(store)
}

async fn ui(uri: Uri) -> Response {
    let file = UI
        .get_file(uri.path().trim_start_matches('/'))
        .unwrap_or_else(|| {
            UI.get_file("index.html")
                .expect("build requires index.html")
        });
    let content_type = mime_guess::from_path(file.path()).first_or_octet_stream();
    (
        [(header::CONTENT_TYPE, content_type.as_ref())],
        file.contents(),
    )
        .into_response()
}
