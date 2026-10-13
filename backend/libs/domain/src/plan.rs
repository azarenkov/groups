#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Free,
    Pro,
    Premium,
}

impl Tier {
    pub fn parse(value: &str) -> Self {
        match value {
            "premium" => Self::Premium,
            "pro" => Self::Pro,
            _ => Self::Free,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Premium => "premium",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_tiers_and_falls_back_to_free() {
        assert_eq!(Tier::parse("premium"), Tier::Premium);
        assert_eq!(Tier::parse("pro"), Tier::Pro);
        assert_eq!(Tier::parse("free"), Tier::Free);
        assert_eq!(Tier::parse("gold"), Tier::Free);
        assert_eq!(Tier::Premium.as_str(), "premium");
    }
}
