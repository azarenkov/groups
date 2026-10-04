use async_trait::async_trait;

pub struct Thumbnail {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
}

#[async_trait]
pub trait Thumbnailer {
    async fn thumbnail(&self, image: Vec<u8>) -> Option<Thumbnail>;
}
