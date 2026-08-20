pub mod dto {
    pub use ::shared::context;
    pub use ::shared::context::RequestContext;
    pub mod requests {
        pub use ::identity::dto::requests::auth;
        pub use ::messaging::dto::requests::{channel, dm, friendship, server};
        pub use ::shared::client_context;
        pub use ::voice::dto::requests::call;
    }
    pub use ::messaging::dto::responses;
}

pub mod ports {
    pub use ::identity::ports::{oauth_verifier, text_moderation, token_issuer, user_state_lookup};
    pub use ::media::ports::{attachment_links_lookup, image_moderation, server_owner_lookup};
    pub use ::messaging::ports::attachment_claims;
    pub use ::platform::ports::{
        event_consumer, file_storage, hasher, mailer, outbox_publisher, outbox_writer,
        voice_service,
    };
    pub use ::voice::ports::{friendship_lookup, voice_webhook_verifier};
}

pub mod usecases {
    pub use ::identity::usecases::auth;
    pub use ::messaging::usecases::{channel, dm, enrichment, friendship, read_state};
    pub use ::voice::usecases::webhook;
    pub mod user {
        pub use ::identity::usecases::user::{get_me, rename, search_users, update_locale};
        pub use ::media::usecases::user::{delete_avatar, get_avatar, update_avatar};
    }
    pub mod attachment {
        pub use ::media::usecases::attachment::{
            cleanup_orphans, delete_attachment, upload_attachment,
        };
        pub use ::messaging::usecases::attachment::{get_attachment, list_message_attachments};
    }
    pub mod server {
        pub use ::media::usecases::server::{get_logo, update_logo};
        pub use ::messaging::usecases::server::{
            accept_server_invite, create_server, decline_server_invite, delete_server,
            get_voice_presence, invite_codes, leave_server, list_sent_server_invites,
            list_server_channels, list_server_members, list_servers_by_user, send_server_invite,
            update_server,
        };
    }
    pub mod call {
        pub use ::messaging::usecases::call::log_call;
        pub use ::voice::usecases::call::{
            accept_call, add_participants, decline_call, get_active_call, get_active_call_with,
            join_call, leave_call, start_call,
        };
    }
}
