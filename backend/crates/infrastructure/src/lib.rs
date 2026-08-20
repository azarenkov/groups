pub use ::platform::{config, hasher, mailer, messaging as nats_messaging, outbox, storage};
pub use ::platform::messaging;

pub mod auth {
    pub use ::identity::auth::{google_verifier, jwt_token_issuer};
}

pub mod moderation {
    pub mod nsfw_detector {
        pub use ::identity::moderation::nsfw_detector::NsfwDetector;
    }
    pub mod nsfw_image {
        pub use ::media::moderation::nsfw_detector::NsfwDetector;
    }
}

pub mod database {
    pub mod postgres {
        pub use ::platform::database::postgres::{connection, error};
        pub mod dto {
            pub use ::identity::database::postgres::dto::{
                user, verification_channel, verification_code, verification_purpose,
            };
            pub use ::messaging::database::postgres::dto::{
                channel, friendship_status, kind, message, muted_participant, server, visibility,
            };
            pub use ::platform::database::postgres::dto::aggregate_type;
        }
        pub mod repositories {
            pub use ::identity::database::postgres::repositories::{
                refresh_token, user, verification_code,
            };
            pub use ::media::database::postgres::repositories::{attachment, media};
            pub use ::messaging::database::postgres::repositories::{
                channel, dm_message, friendship, message, read_state, server,
            };
            pub use ::platform::database::postgres::repositories::{
                outbox_record, outbox_writer, user_profile,
            };
            pub use ::voice::database::postgres::repositories::call;
        }
    }
}

pub mod media_client {
    pub use ::identity::media_client::user_state;
    pub use ::media::media_client::{links, owner_lookup};
    pub use ::messaging::media_client::claims;
    pub use ::voice::media_client::friendship;
}

pub mod voice {
    pub use ::platform::voice::livekit_service;
    pub use ::voice::voice::livekit_webhook_receiver;
}
