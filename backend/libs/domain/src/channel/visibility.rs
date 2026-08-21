use serde::Serialize;

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelVisibility {
    Public,
    Private,
}
