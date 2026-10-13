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
    pub const FREE: Self = Self { width: 1280, height: 720, frame_rate: 60, max_bitrate: 4_000_000 };
    pub const PRO: Self = Self { width: 2560, height: 1440, frame_rate: 60, max_bitrate: 12_000_000 };
    pub const PREMIUM: Self = Self { width: 3840, height: 2160, frame_rate: 60, max_bitrate: 20_000_000 };

    pub fn for_tier(tier: &str) -> Self {
        match tier {
            "premium" => Self::PREMIUM,
            "pro" => Self::PRO,
            _ => Self::FREE,
        }
    }

    pub fn allows(&self, width: u32, height: u32, bitrate: u64) -> bool {
        let (long, short) = (width.max(height), width.min(height));
        long <= self.width && short <= self.height && bitrate <= self.max_bitrate + self.max_bitrate / 4
    }

    pub fn to_attribute(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tier_gets_its_ceiling() {
        assert_eq!(StreamCeiling::for_tier("free"), StreamCeiling::FREE);
        assert_eq!(StreamCeiling::for_tier("pro"), StreamCeiling::PRO);
        assert_eq!(StreamCeiling::for_tier("premium"), StreamCeiling::PREMIUM);
        assert_eq!(StreamCeiling::for_tier("unknown"), StreamCeiling::FREE);
    }

    #[test]
    fn a_track_above_the_ceiling_is_not_allowed() {
        let free = StreamCeiling::FREE;
        assert!(free.allows(1280, 720, 4_000_000));
        assert!(free.allows(720, 1280, 3_000_000));
        assert!(!free.allows(1280, 800, 0));
        assert!(!free.allows(1920, 1080, 4_000_000));
        assert!(!free.allows(1280, 720, 6_000_000));
        assert!(StreamCeiling::PREMIUM.allows(3840, 2160, 20_000_000));
    }

    #[test]
    fn the_ceiling_travels_as_a_camel_case_attribute() {
        assert_eq!(
            StreamCeiling::FREE.to_attribute(),
            r#"{"width":1280,"height":720,"frameRate":60,"maxBitrate":4000000}"#
        );
    }
}
