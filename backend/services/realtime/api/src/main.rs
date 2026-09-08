use std::error::Error;

mod config;
mod dispatcher;
mod websocket;
mod ws;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = api_common::health::run_probe_from_args() {
        std::process::exit(code);
    }
    infrastructure_common::observability::init("realtime");
    tokio::try_join!(dispatcher::run(), websocket::run())?;
    Ok(())
}
