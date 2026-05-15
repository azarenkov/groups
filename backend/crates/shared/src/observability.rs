use tracing_log::LogTracer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

pub fn init(service_name: &'static str) {
    let _ = LogTracer::init();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let json_output = std::env::var("RIFT_LOG_JSON")
        .map(|v| v == "1")
        .unwrap_or(false);

    let builder = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_span_events(FmtSpan::NONE);

    if json_output {
        let subscriber = builder
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    } else {
        let subscriber = builder.pretty().finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }

    tracing::info!(service = service_name, "tracing initialized");
}
