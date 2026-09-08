use std::error::Error;

mod config;
mod dispatcher;
mod websocket;
mod ws;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    infrastructure_common::observability::init("realtime");
    let shutdown = api_common::shutdown::ShutdownSignal::install();
    tokio::try_join!(dispatcher::run(shutdown.clone()), websocket::run(shutdown))?;
    Ok(())
}
