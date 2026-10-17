use std::sync::Arc;

use infrastructure_common::health::Heartbeat;
use infrastructure_common::messaging::nats::runner::{ConsumerSpec as RunnerSpec, Delivery, Event, Lifecycle, run_consumer as run};
use ports::event_consumer::StartPosition;
use realtime_core::ports::dm_history::DmHistory;
use realtime_core::{
    ports::directory::RecipientDirectory, ports::projection::RealtimeProjection, ports::publisher::RealtimePublisher,
    usecases::dispatch_event::{DispatchEvent, EventSequence},
};

use super::sequence_gate::SequenceGate;

#[derive(Clone)]
pub struct ConsumerSpec {
    pub stream: String,
    pub consumer_name: String,
    pub filter_subject: String,
    pub dlq_subject: String,
    pub max_deliver: u32,
    pub start_position: StartPosition,
}

impl ConsumerSpec {
    fn runner_spec(&self) -> RunnerSpec {
        RunnerSpec {
            stream: self.stream.clone(),
            consumer_name: self.consumer_name.clone(),
            subjects: vec![self.filter_subject.clone()],
            dlq_subject: self.dlq_subject.clone(),
            start_position: self.start_position,
            delivery: Delivery { max_deliver: self.max_deliver, ..Delivery::DEFAULT },
        }
    }
}

pub async fn run_consumer<D: RecipientDirectory + RealtimeProjection + 'static, P: RealtimePublisher + 'static, B: DmHistory + 'static>(
    jetstream: async_nats::jetstream::Context,
    spec: ConsumerSpec,
    dispatch: Arc<DispatchEvent<D, P, B>>,
    shutdown: tokio::sync::watch::Receiver<bool>,
    heartbeat: Heartbeat,
    gate: Option<Arc<SequenceGate>>,
) {
    let lifecycle = Lifecycle::default().with_shutdown(shutdown, heartbeat);
    run(jetstream, spec.runner_spec(), lifecycle, move |event: Event| {
        let (dispatch, gate) = (dispatch.clone(), gate.clone());
        async move {
            let sequence = event
                .sequence
                .map(|s| EventSequence { partition: s.partition, seq: s.seq, members: s.members });
            let gated = gate.as_ref().zip(sequence.clone());
            let mut in_order = true;
            if let Some((gate, s)) = &gated {
                in_order = gate.wait_turn(&s.partition, s.seq).await;
            }
            let dispatched = if in_order {
                dispatch.execute_event(&event.subject, &event.payload, sequence, event.event_id.as_deref()).await
            } else {
                tracing::debug!(subject = %event.subject, "Event arrived after a later one of its partition; applying it without publishing");
                dispatch.execute_unpublished(&event.subject, &event.payload).await
            };
            match dispatched {
                Ok(_) => {
                    if let Some((gate, s)) = &gated {
                        gate.done(&s.partition, s.seq);
                    }
                    Ok(())
                }
                Err(e) => {
                    if let Some((gate, s)) = &gated {
                        gate.release(&s.partition, s.seq);
                    }
                    Err(e.into())
                }
            }
        }
    })
    .await;
}
