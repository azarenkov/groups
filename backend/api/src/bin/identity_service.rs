use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    api::identity::server::run().await
}
