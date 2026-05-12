use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use actix_web::{App, HttpResponse, HttpServer, web};

#[derive(Clone, Default)]
pub struct Readiness(Arc<AtomicBool>);

impl Readiness {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_ready(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_ready(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub async fn serve(port: u16, readiness: Readiness) -> io::Result<()> {
    let readiness_data = web::Data::new(readiness);
    tracing::info!(port, "Health server listening");
    HttpServer::new(move || {
        App::new()
            .app_data(readiness_data.clone())
            .route("/livez", web::get().to(livez))
            .route("/readyz", web::get().to(readyz))
    })
    .workers(1)
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

async fn livez() -> HttpResponse {
    HttpResponse::Ok().finish()
}

async fn readyz(readiness: web::Data<Readiness>) -> HttpResponse {
    if readiness.is_ready() {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::ServiceUnavailable().finish()
    }
}
