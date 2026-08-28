mod dto;
mod http_errors;
mod bootstrap;
mod attachments;
mod calls;
mod channel;
mod dm;
mod friendship;
mod home;
mod internal;
mod notifications;
mod presence;
mod read_state;
mod roles;
mod routes;
mod server;
mod state;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    crate::bootstrap::run().await
}
