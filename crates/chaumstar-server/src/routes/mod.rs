//! Router assembly.

use std::path::PathBuf;

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::services::{ServeDir, ServeFile};

use crate::state::AppState;

mod health;
mod keysets;
mod mints;
mod reviews;
mod transparency;

pub fn build_router(state: AppState) -> Router {
    let mut router = Router::new()
        .route("/api/v1/health", get(health::handler))
        .route("/api/v1/keysets", get(keysets::list))
        .route("/api/v1/keysets/{kid}", get(keysets::get_one))
        .route("/api/v1/mints", post(mints::create))
        .route("/api/v1/reviews", post(reviews::create).get(reviews::list))
        .route("/api/v1/reviews/{hpk}", get(reviews::get_one))
        .route("/api/v1/registry-key", get(transparency::registry_key))
        .route("/api/v1/sth", get(transparency::sth))
        .with_state(state);

    // Optional static-asset serving for the SPA. When CHAUMSTAR_STATIC_DIR is
    // set, every non-API path falls back to ServeDir, with index.html acting
    // as the SPA fallback so client-side routes 404 gracefully.
    if let Ok(dir) = std::env::var("CHAUMSTAR_STATIC_DIR") {
        let dir = PathBuf::from(dir);
        let index = dir.join("index.html");
        let serve = ServeDir::new(&dir).fallback(ServeFile::new(index));
        router = router.fallback_service(serve);
        tracing::info!("static dir: {}", dir.display());
    } else {
        tracing::info!("static dir: <not set> (API only)");
    }

    router
}
