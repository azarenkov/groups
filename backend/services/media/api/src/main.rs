mod attachment;
mod http_errors;
mod internal;
mod avatar;
mod banner;
mod sticker;
mod projection;
mod storage_release;
mod user_projection;
mod routes;
mod server;
mod state;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = api_common::health::run_probe_from_args() {
        std::process::exit(code);
    }
    crate::server::run().await
}
