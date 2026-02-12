use std::error::Error;

use crate::server::run;

pub mod config;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod server;
pub mod shared;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    run().await?;
    Ok(())
}
