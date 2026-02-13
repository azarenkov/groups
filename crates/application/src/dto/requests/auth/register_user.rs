#[derive(Debug)]
pub struct RegisterUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

impl RegisterUserRequest {
    pub fn new(name: String, email: String, password: String) -> Self {
        Self {
            name,
            email,
            password,
        }
    }
}
