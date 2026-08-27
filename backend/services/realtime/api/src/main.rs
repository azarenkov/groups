use std::error::Error;

mod config;
mod dispatcher;
mod websocket;
mod ws;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    infrastructure_common::observability::init("realtime");
    tokio::try_join!(dispatcher::run(), websocket::run())?;
    Ok(())
}
