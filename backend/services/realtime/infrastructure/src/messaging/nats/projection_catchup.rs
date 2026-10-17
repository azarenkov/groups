use std::sync::Arc;

use infrastructure_common::health::Heartbeat;
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Event, Lifecycle, run_consumer};
use realtime_core::{ports::{dm_history::DmHistory, projection::RealtimeProjection}, usecases::apply_projection::ApplyProjection};

pub struct CatchupSpec {
    pub stream: String,
    pub consumer_name: String,
    pub filter_subject: String,
    pub dlq_subject: String,
}

pub async fn run_catchup<R: RealtimeProjection + 'static, B: DmHistory + 'static>(
    jetstream: async_nats::jetstream::Context,
    spec: CatchupSpec,
    apply: Arc<ApplyProjection<R, B>>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    heartbeat: Heartbeat,
) {
    let runner_spec = ConsumerSpec {
        stream: spec.stream,
        consumer_name: spec.consumer_name,
        subjects: vec![spec.filter_subject],
        dlq_subject: spec.dlq_subject,
        ..ConsumerSpec::new("", "", "", Vec::new())
    };
    let lifecycle = Lifecycle::default().with_shutdown(shutdown, heartbeat);
    run_consumer(jetstream, runner_spec, lifecycle, move |event: Event| {
        let apply = apply.clone();
        async move { apply.execute(&event.subject, &event.payload).await.map_err(Into::into) }
    })
    .await;
}
