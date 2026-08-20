use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    shared::observability::init("realtime");
    tokio::try_join!(event_dispatcher::run(), websocket::run())?;
    Ok(())
}
