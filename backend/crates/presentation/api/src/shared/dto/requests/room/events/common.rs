use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Room {
    pub name: String,
    pub metadata: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub identity: String,
    pub name: Option<String>,
    pub joined_at: Option<String>,
}
