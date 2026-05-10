use serde::Serialize;

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub enum ChannelVisibility {
    Public,
    Private,
}
