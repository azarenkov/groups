use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub enum ChannelType {
    Public,
    Private,
}
