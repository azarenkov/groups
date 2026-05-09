use std::fmt;

#[derive(Debug)]
pub struct ChannelToken {
    value: String,
}

impl ChannelToken {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for ChannelToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
