use tokio::sync::broadcast;

pub struct AppState {
    pub broadcaster: broadcast::Sender<String>,
}

impl AppState {
    pub fn new(buffer: usize) -> Self {
        let (tx, _rx) = broadcast::channel(buffer);
        Self { broadcaster: tx }
    }
}
