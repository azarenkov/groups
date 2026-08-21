mod dto;
mod bootstrap;
mod attachments;
mod calls;
mod channel;
mod dm;
mod friendship;
mod internal;
mod presence;
mod read_state;
mod routes;
mod server;
mod state;
mod webhook_projection;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    crate::bootstrap::run().await
}
