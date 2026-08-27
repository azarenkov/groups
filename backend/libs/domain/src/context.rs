#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClientContext {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}
