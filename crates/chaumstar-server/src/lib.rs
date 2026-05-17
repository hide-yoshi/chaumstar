//! chaumstar-server
//!
//! HTTP server exposing the chaumstar Issuer (mint) and Registry APIs.
//! See `PROTOCOL.md` / `DEMO.md` in the workspace root for the higher-level
//! design.

mod error;
mod routes;
mod state;

pub use error::ApiError;
pub use routes::build_router;
pub use state::{AppState, InsertError, ReviewStore};
