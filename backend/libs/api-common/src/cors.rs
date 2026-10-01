use actix_cors::Cors;

pub fn allowed_origins() -> Option<Vec<String>> {
    let origins: Vec<String> = std::env::var("CORS_ALLOWED_ORIGINS")
        .ok()?
        .split(',')
        .map(|origin| origin.trim().trim_end_matches('/').to_string())
        .filter(|origin| !origin.is_empty())
        .collect();
    (!origins.is_empty()).then_some(origins)
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
