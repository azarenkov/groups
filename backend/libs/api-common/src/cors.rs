use actix_cors::Cors;

fn normalize_origin(origin: &str) -> String {
    let origin = origin.trim();
    if origin.ends_with("://") {
        return origin.to_string();
    }
    origin.trim_end_matches('/').to_string()
}

pub fn parse_origins(raw: &str) -> Option<Vec<String>> {
    let origins: Vec<String> = raw
        .split(',')
        .map(normalize_origin)
        .filter(|origin| !origin.is_empty())
        .collect();
    (!origins.is_empty()).then_some(origins)
}

pub fn allowed_origins() -> Option<Vec<String>> {
    parse_origins(&std::env::var("CORS_ALLOWED_ORIGINS").ok()?)
}

pub fn cors() -> Cors {
    let base = match allowed_origins() {
        Some(origins) => origins
            .into_iter()
            .fold(Cors::default(), |cors, origin| cors.allowed_origin(&origin)),
        None => Cors::default().allow_any_origin(),
    };
    base.allow_any_method()
        .allow_any_header()
        .expose_headers(["x-request-id"])
        .max_age(3600)
}

#[cfg(test)]
mod tests {
    use super::parse_origins;

    #[test]
    fn keeps_scheme_only_origins_and_trims_paths() {
        assert_eq!(
            parse_origins(" https://app.rift.example.com/ ,file://, null ,"),
            Some(vec![
                "https://app.rift.example.com".to_string(),
                "file://".to_string(),
                "null".to_string(),
            ])
        );
        assert_eq!(parse_origins(" , "), None);
    }
}
