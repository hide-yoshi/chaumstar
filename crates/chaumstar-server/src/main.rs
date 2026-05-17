//! chaumstar-server binary entry point.

use chaumstar_core::Issuer;
use chaumstar_server::{AppState, build_router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chaumstar_server=info,tower_http=info".into()),
        )
        .init();

    // Bootstrap: spin up one demo issuer / merchant pair.
    let state = AppState::new();
    state.register_issuer(Issuer::generate("Bean & Beam Coffee", "Bean & Beam 下北沢"));

    let app = build_router(state);
    let addr: SocketAddr = std::env::var("CHAUMSTAR_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()
        .expect("CHAUMSTAR_BIND must be a valid SocketAddr");
    tracing::info!("chaumstar-server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind tcp listener");
    axum::serve(listener, app).await.expect("server");
}
