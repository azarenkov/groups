use domain::{entites::room::RoomError, repositories::voice_room::VoiceRoomRepository};

#[derive(Debug, thiserror::Error)]
pub enum CreateVoiceRoomError {
    #[error("Domain error: {0}")]
    Domain(#[from] RoomError),
}

pub struct CreateVoiceRoom<T>
where
    T: VoiceRoomRepository,
{
    voice_room_repository: T,
}

impl<T> CreateVoiceRoom<T>
where
    T: VoiceRoomRepository,
{
    pub fn new(voice_room_repository: T) -> Self {
        Self {
            voice_room_repository,
        }
    }

    async fn execute(&self) -> Result<(), CreateVoiceRoomError> {
        Ok(())
    }
}
