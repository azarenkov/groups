use messaging_domain::{channel::Channel, channel::ChannelError, channel::kind::ChannelKind, channel::name::ChannelName, channel::visibility::ChannelVisibility, community::invite_link::InviteLink};

const DEFAULT_CHANNEL_NAME: &str = "general";

use crate::usecases::community::invite_links::create_with_fresh_token;
use std::sync::Arc;

use messaging_domain::{events::MessagingEvent, community::Community, community::description::CommunityDescription, community::name::CommunityName, community::unit_of_work::CommunityUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use ports::text_moderation::{TextModeration, TextModerationError};

use crate::dto::requests::community::create_community::CreateCommunityRequest;
use crate::usecases::community::text_policy::{CommunityTextField, ensure_community_text_allowed};

#[derive(Debug, thiserror::Error)]
pub enum CreateCommunityError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Account is inactive")]
    AccountInactive,
    #[error("Community name is not allowed")]
    NameNotAllowed,
    #[error("Community description is not allowed")]
    DescriptionNotAllowed,
    #[error("Text moderation unavailable: {0}")]
    ModerationUnavailable(String),
    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),
}

pub struct CreateCommunity<W, TM>
where
    W: CommunityUnitOfWork,
    TM: TextModeration,
{
    unit_of_work: Arc<W>,
    text_moderation: Arc<TM>,
}

impl<W, TM> CreateCommunity<W, TM>
where
    W: CommunityUnitOfWork,
    TM: TextModeration,
{
    pub fn new(unit_of_work: Arc<W>, text_moderation: Arc<TM>) -> Self {
        Self {
            unit_of_work,
            text_moderation,
        }
    }

    pub async fn execute(
        &self,
        request: CreateCommunityRequest,
        owner_id: UserId,
    ) -> Result<Community, CreateCommunityError> {
        match ensure_community_text_allowed(
            self.text_moderation.as_ref(),
            &request.name,
            request.description.as_deref(),
        )
        .await
        {
            Ok(()) => {}
            Err((CommunityTextField::Name, TextModerationError::Rejected)) => {
                return Err(CreateCommunityError::NameNotAllowed);
            }
            Err((CommunityTextField::Description, TextModerationError::Rejected)) => {
                return Err(CreateCommunityError::DescriptionNotAllowed);
            }
            Err((_, TextModerationError::Unavailable(e))) => {
                return Err(CreateCommunityError::ModerationUnavailable(e));
            }
        }
        let mut community = Community::create(
            CommunityName::new(request.name),
            owner_id,
            request.description.map(CommunityDescription::new),
            request.accent_color,
        );

        let request_id = request.ctx.request_id.clone();
        let events = community.pull_events();

        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let mut tx = self.unit_of_work.begin(community.id()).await?;
        tx.lock_user(community.owner_id()).await?;
        if tx.is_user_deleted(community.owner_id()).await? {
            return Err(CreateCommunityError::AccountInactive);
        }
        tx.insert_community(&community).await?;
        let now = chrono::Utc::now();
        let mut general = Channel::create(
            *community.id(),
            owner_id,
            ChannelName::new(DEFAULT_CHANNEL_NAME.to_string()),
            ChannelKind::Text,
            ChannelVisibility::Public,
            None,
            now,
        )?
        .with_default(true);
        let mut events = events;
        let owner_recipients = [owner_id];
        events.extend(
            general
                .pull_events()
                .into_iter()
                .map(|event| MessagingEvent::from(event.with_recipients(&owner_recipients))),
        );
        tx.insert_channel(&general).await?;
        create_with_fresh_token(&mut *tx, || InviteLink::primary(*community.id(), owner_id, chrono::Utc::now())).await?;
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(community)
    }
}
