use domain::value_objects::room::name::RoomName;

#[derive(Debug)]
pub struct GenerateTokenRequest {
    pub room_name: RoomName,
}
