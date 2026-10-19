use serde::Serialize;

pub const STREAM_CEILING_ATTRIBUTE: &str = "rift.streamCeiling";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamCeiling {
    pub width: u32,
    pub height: u32,
    pub frame_rate: u32,
    pub max_bitrate: u64,
}

impl StreamCeiling {
    pub const MAX: Self = Self { width: 3840, height: 2160, frame_rate: 60, max_bitrate: 20_000_000 };

    pub fn to_attribute(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ceiling_travels_as_a_camel_case_attribute() {
        assert_eq!(
            StreamCeiling::MAX.to_attribute(),
            r#"{"width":3840,"height":2160,"frameRate":60,"maxBitrate":20000000}"#
        );
    }
}
