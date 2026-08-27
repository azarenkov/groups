mod attachment;
mod http_errors;
mod internal;
mod avatar;
mod projection;
mod routes;
mod server;
mod state;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    crate::server::run().await
}
