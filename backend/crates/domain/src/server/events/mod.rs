use serde::Serialize;

use crate::room::events::RoomEvent;

#[derive(Debug, Serialize)]
pub enum ServerEvent {
    RoomEvent(RoomEvent),
}
