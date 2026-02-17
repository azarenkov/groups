use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Room {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub identity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<u64>,
}
