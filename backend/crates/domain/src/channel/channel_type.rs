use serde::Serialize;

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub enum ChannelType {
    Public,
    Private,
}
