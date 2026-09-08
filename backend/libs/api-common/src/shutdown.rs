use std::{io, time::Duration};

use actix_web::dev::Server;

use crate::health::Readiness;

const DEFAULT_DRAIN_MS: u64 = 3_000;
const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 25;

#[derive(Debug, Clone, Copy)]
pub struct ShutdownConfig {
    pub drain_delay: Duration,
    pub shutdown_timeout: Duration,
}

impl ShutdownConfig {
    pub fn from_env() -> Self {
        let read = |name: &str| {
            std::env::var(name)
                .ok()
                .and_then(|raw| raw.trim().parse::<u64>().ok())
        };
        Self {
            drain_delay: Duration::from_millis(read("SHUTDOWN_DRAIN_MS").unwrap_or(DEFAULT_DRAIN_MS)),
            shutdown_timeout: Duration::from_secs(
                read("SHUTDOWN_TIMEOUT_SECS").unwrap_or(DEFAULT_SHUTDOWN_TIMEOUT_SECS),
            ),
        }
    }
}

#[cfg(unix)]
pub async fn wait_for_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = match signal(SignalKind::terminate()) {
        Ok(stream) => Some(stream),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to install SIGTERM handler; only SIGINT will be handled");
            None
        }
    };
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(e) = result {
                tracing::warn!(error = %e, "Failed to listen for SIGINT");
                std::future::pending::<()>().await;
            }
            tracing::info!("SIGINT received");
        }
        _ = async {
            match terminate.as_mut() {
                Some(stream) => {
                    stream.recv().await;
                }
                None => std::future::pending::<()>().await,
            }
        } => {
            tracing::info!("SIGTERM received");
        }
    }
}

#[cfg(not(unix))]
pub async fn wait_for_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::warn!(error = %e, "Failed to listen for SIGINT");
        std::future::pending::<()>().await;
    }
    tracing::info!("SIGINT received");
}

pub async fn drain(readiness: &Readiness, config: &ShutdownConfig) {
    readiness.mark_not_ready();
    tracing::info!(
        drain_ms = config.drain_delay.as_millis() as u64,
        "Marked not ready; draining before shutdown"
    );
    tokio::time::sleep(config.drain_delay).await;
}

pub async fn serve(server: Server, readiness: Readiness, config: ShutdownConfig) -> io::Result<()> {
    let handle = server.handle();
    let task = actix_web::rt::spawn(server);
    tokio::pin!(task);
    tokio::select! {
        result = &mut task => {
            return result.map_err(io::Error::other)?;
        }
        _ = wait_for_signal() => {}
    }
    drain(&readiness, &config).await;
    handle.stop(true).await;
    task.await.map_err(io::Error::other)?
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sigterm_resolves_wait_for_signal() {
        let waiter = tokio::spawn(wait_for_signal());
        tokio::time::sleep(Duration::from_millis(50)).await;
        let status = std::process::Command::new("kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .expect("kill");
        assert!(status.success());
        tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("signal handled")
            .expect("waiter task");
    }

    #[test]
    fn shutdown_config_reads_env_with_defaults() {
        let config = ShutdownConfig::from_env();
        assert_eq!(config.drain_delay, Duration::from_millis(DEFAULT_DRAIN_MS));
        assert_eq!(config.shutdown_timeout, Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECS));
    }
}
