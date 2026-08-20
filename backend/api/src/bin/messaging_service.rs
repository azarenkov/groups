use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    api::messaging::bootstrap::run().await
}
