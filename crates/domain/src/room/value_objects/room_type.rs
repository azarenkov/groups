use serde::Serialize;

#[derive(Debug, PartialEq, Serialize, Clone, Copy)]
pub enum RoomType {
    Public,
    Private,
}
