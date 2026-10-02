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
        Some(origins) => Cors::default()
            .allowed_origin_fn(move |origin, _| origins.iter().any(|allowed| origin.as_bytes() == allowed.as_bytes())),
        None => Cors::default().allow_any_origin(),
    };
    base.allow_any_method()
        .allow_any_header()
        .expose_headers(["x-request-id"])
        .max_age(3600)
}

#[cfg(test)]
mod tests {
    use super::{cors, parse_origins};
    use actix_web::{App, HttpResponse, http::header, test as actix_test, web};

    #[actix_web::test]
    async fn scheme_only_and_null_origins_start_and_are_allowed() {
        unsafe { std::env::set_var("CORS_ALLOWED_ORIGINS", "https://app.rift.example.com/,file://,null") };
        let app = actix_test::init_service(
            App::new()
                .wrap(cors())
                .route("/", web::get().to(HttpResponse::Ok)),
        )
        .await;
        for (origin, allowed) in [
            ("https://app.rift.example.com", true),
            ("file://", true),
            ("null", true),
            ("https://evil.example.com", false),
        ] {
            let request = actix_test::TestRequest::get()
                .uri("/")
                .insert_header((header::ORIGIN, origin))
                .to_request();
            let response = actix_test::call_service(&app, request).await;
            assert_eq!(
                response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_some(),
                allowed,
                "{origin}"
            );
        }
    }

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
