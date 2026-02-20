use serde::Serialize;
use uuid::Uuid;

pub trait DomainEvent: Serialize {
    fn event_category(&self) -> &'static str;
    fn event_type(&self) -> &'static str;
    fn aggregate_id(&self) -> Uuid;
    fn parent_id(&self) -> Option<Uuid> {
        None
    }
    fn event_name(&self) -> String {
        format!("{}.{}", self.event_category(), self.event_type())
    }
}
