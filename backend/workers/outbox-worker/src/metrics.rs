use std::collections::BTreeMap;
use std::fmt::Write;
use std::sync::{Arc, RwLock};

use actix_web::{HttpResponse, web};
use infrastructure_common::outbox::dispatcher::OutboxBacklog;

type Gauge = (&'static str, &'static str, fn(&OutboxBacklog) -> f64);

#[derive(Clone, Default)]
pub struct OutboxMetrics {
    backlogs: Arc<RwLock<BTreeMap<String, OutboxBacklog>>>,
}

impl OutboxMetrics {
    pub fn record(&self, source: &str, backlog: OutboxBacklog) {
        if let Ok(mut backlogs) = self.backlogs.write() {
            backlogs.insert(source.to_string(), backlog);
        }
    }

    pub fn render(&self) -> String {
        let backlogs = match self.backlogs.read() {
            Ok(backlogs) => backlogs.clone(),
            Err(_) => return String::new(),
        };
        let mut out = String::new();
        let gauges: [Gauge; 3] = [
            (
                "rift_outbox_pending_events",
                "Outbox events not yet published",
                |b| b.pending as f64,
            ),
            (
                "rift_outbox_stuck_events",
                "Outbox events that used up their publish attempts and hold back their partition",
                |b| b.stuck as f64,
            ),
            (
                "rift_outbox_oldest_pending_age_seconds",
                "Age of the oldest unpublished outbox event",
                |b| b.oldest_pending_secs,
            ),
        ];
        for (name, help, value) in gauges {
            let _ = writeln!(out, "# HELP {name} {help}");
            let _ = writeln!(out, "# TYPE {name} gauge");
            for (source, backlog) in &backlogs {
                let _ = writeln!(out, "{name}{{source=\"{source}\"}} {}", value(backlog));
            }
        }
        out
    }

    pub fn configure(&self) -> impl Fn(&mut web::ServiceConfig) + Send + Clone + 'static {
        let metrics = self.clone();
        move |cfg: &mut web::ServiceConfig| {
            let metrics = metrics.clone();
            cfg.service(web::resource("/metrics").to(move || {
                let body = metrics.render();
                async move {
                    HttpResponse::Ok()
                        .content_type("text/plain; version=0.0.4")
                        .body(body)
                }
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_gauge_per_source() {
        let metrics = OutboxMetrics::default();
        metrics.record(
            "messaging",
            OutboxBacklog {
                pending: 3,
                stuck: 1,
                oldest_pending_secs: 42.5,
            },
        );
        metrics.record("identity", OutboxBacklog::default());
        let text = metrics.render();
        assert!(text.contains("# TYPE rift_outbox_stuck_events gauge"));
        assert!(text.contains("rift_outbox_pending_events{source=\"messaging\"} 3"));
        assert!(text.contains("rift_outbox_stuck_events{source=\"messaging\"} 1"));
        assert!(text.contains("rift_outbox_stuck_events{source=\"identity\"} 0"));
        assert!(text.contains("rift_outbox_oldest_pending_age_seconds{source=\"messaging\"} 42.5"));
    }

    #[test]
    fn renders_only_headers_before_the_first_sample() {
        let text = OutboxMetrics::default().render();
        assert_eq!(text.lines().count(), 6);
    }
}
