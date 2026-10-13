mod call;
mod evictions;
mod http_errors;
mod internal;
mod projection;
mod routes;
mod server;
mod state;
mod stream_sweep;
mod webhook;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = api_common::health::run_probe_from_args() {
        std::process::exit(code);
    }
    crate::server::run().await
}
