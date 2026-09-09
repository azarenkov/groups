use domain::user::id::UserId;
use messaging_domain::{channel::Channel, channel::moderation::Publication, channel::moderation::Subscription};
use ports::voice_service::ParticipantAccess;

pub fn participant_access(channel: &Channel, user_id: &UserId) -> ParticipantAccess {
    ParticipantAccess {
        can_publish_microphone: channel.publication_for(user_id) == Publication::Full,
        can_subscribe: channel.subscription_for(user_id) == Subscription::Full,
    }
}
