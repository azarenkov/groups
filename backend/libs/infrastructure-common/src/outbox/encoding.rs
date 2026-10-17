use ports::repository::RepositoryError;
use serde::Serialize;

pub fn encode<T: Serialize>(value: &T) -> Result<serde_json::Value, RepositoryError> {
    serde_json::to_value(value).map_err(|e| RepositoryError::InvalidData(e.to_string()))
}

pub fn snake_case<T: Serialize>(value: &T) -> Result<String, RepositoryError> {
    encode(value)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| RepositoryError::InvalidData("expected string enum".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Kind {
        ScreenShare,
    }

    #[test]
    fn enums_serialise_to_their_snake_case_name_and_structs_are_rejected() {
        assert_eq!(snake_case(&Kind::ScreenShare).unwrap(), "screen_share");
        assert!(matches!(snake_case(&vec![1]), Err(RepositoryError::InvalidData(_))));
        assert_eq!(encode(&3).unwrap(), serde_json::json!(3));
    }
}
