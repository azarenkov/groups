use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    api::media::server::run().await
}
