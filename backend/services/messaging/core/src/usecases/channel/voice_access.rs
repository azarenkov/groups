use domain::user::id::UserId;
use messaging_domain::{channel::Channel, channel::moderation::Publication, channel::moderation::Subscription};
use ports::voice_service::ParticipantAccess;

pub fn participant_access(channel: &Channel, user_id: &UserId) -> ParticipantAccess {
    let can_subscribe = channel.subscription_for(user_id) == Subscription::Full;
    ParticipantAccess {
        can_publish_microphone: can_subscribe && channel.publication_for(user_id) == Publication::Full,
        can_subscribe,
    }
}
