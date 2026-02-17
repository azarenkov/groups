use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub enum RoomType {
    Public,
    Private,
}
