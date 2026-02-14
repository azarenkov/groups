#[derive(Debug)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: Option<String>,
}
