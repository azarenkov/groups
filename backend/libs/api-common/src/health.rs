use std::{
    future::Future,
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use actix_web::{App, HttpResponse, HttpServer, dev::Server, web};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type Probe = Arc<dyn Fn() -> BoxFuture<'static, bool> + Send + Sync>;

#[derive(Clone)]
pub struct Readiness {
    started: Arc<AtomicBool>,
    probes: Arc<Vec<Probe>>,
}

impl Readiness {
    pub fn builder() -> ReadinessBuilder {
        ReadinessBuilder { probes: Vec::new() }
    }

    pub fn mark_ready(&self) {
        self.started.store(true, Ordering::Release);
    }

    pub async fn check(&self) -> bool {
        if !self.started.load(Ordering::Acquire) {
            return false;
        }
        for probe in self.probes.iter() {
            if !probe().await {
                return false;
            }
        }
        true
    }
}

pub struct ReadinessBuilder {
    probes: Vec<Probe>,
}

impl ReadinessBuilder {
    pub fn add_probe<F, Fut>(mut self, probe: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        let probe = Arc::new(probe);
        self.probes.push(Arc::new(move || {
            let probe = probe.clone();
            Box::pin(async move { probe().await }) as BoxFuture<'static, bool>
        }));
        self
    }

    pub fn build(self) -> Readiness {
        Readiness {
            started: Arc::new(AtomicBool::new(false)),
            probes: Arc::new(self.probes),
        }
    }
}

pub fn bind(port: u16, readiness: Readiness) -> io::Result<Server> {
    let readiness_data = web::Data::new(readiness);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(readiness_data.clone())
            .service(web::resource("/livez").to(livez))
            .service(web::resource("/readyz").to(readyz))
    })
    .workers(1)
    .bind(("0.0.0.0", port))?
    .run();
    tracing::info!(port, "Health server listening");
    Ok(server)
}

pub async fn livez() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub async fn readyz(readiness: web::Data<Readiness>) -> HttpResponse {
    if readiness.check().await {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::ServiceUnavailable().finish()
    }
}
