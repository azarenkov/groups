use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, community::description::CommunityDescription, community::name::CommunityName, community::policy::PolicyError, community::unit_of_work::CommunityUnitOfWork};
use ports::repository::RepositoryError;
use ports::text_moderation::{TextModeration, TextModerationError};

use crate::dto::requests::community::update_community::UpdateCommunityRequest;
use crate::usecases::community::text_policy::{CommunityTextField, ensure_community_text_allowed};

#[derive(Debug, thiserror::Error)]
pub enum UpdateCommunityError {
    #[error("Community not found")]
    CommunityNotFound,
    #[error("Only the community owner can edit it")]
    NotOwner,
    #[error("Community name already taken")]
    NameTaken,
    #[error("Community name is not allowed")]
    NameNotAllowed,
    #[error("Community description is not allowed")]
    DescriptionNotAllowed,
    #[error("Text moderation unavailable: {0}")]
    ModerationUnavailable(String),
    #[error("Repository error: {0}")]
    Repository(RepositoryError),
}

impl From<RepositoryError> for UpdateCommunityError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::AlreadyExists => UpdateCommunityError::NameTaken,
            RepositoryError::NotFound => UpdateCommunityError::CommunityNotFound,
            other => UpdateCommunityError::Repository(other),
        }
    }
}

impl From<PolicyError> for UpdateCommunityError {
    fn from(_: PolicyError) -> Self {
        UpdateCommunityError::NotOwner
    }
}

pub struct UpdateCommunity<W: CommunityUnitOfWork, TM: TextModeration> {
    unit_of_work: Arc<W>,
    text_moderation: Arc<TM>,
}

impl<W: CommunityUnitOfWork, TM: TextModeration> UpdateCommunity<W, TM> {
    pub fn new(unit_of_work: Arc<W>, text_moderation: Arc<TM>) -> Self {
        Self {
            unit_of_work,
            text_moderation,
        }
    }

    pub async fn execute(&self, request: UpdateCommunityRequest) -> Result<(), UpdateCommunityError> {
        let UpdateCommunityRequest {
            community_id,
            owner_id,
            name,
            description,
            ctx,
        } = request;
        {
            let mut precheck = self.unit_of_work.begin(&community_id).await?;
            precheck.load_community().await?.ensure_can_manage(&owner_id)?;
        }
        match ensure_community_text_allowed(self.text_moderation.as_ref(), &name, description.as_deref())
            .await
        {
            Ok(()) => {}
            Err((CommunityTextField::Name, TextModerationError::Rejected)) => {
                return Err(UpdateCommunityError::NameNotAllowed);
            }
            Err((CommunityTextField::Description, TextModerationError::Rejected)) => {
                return Err(UpdateCommunityError::DescriptionNotAllowed);
            }
            Err((_, TextModerationError::Unavailable(e))) => {
                return Err(UpdateCommunityError::ModerationUnavailable(e));
            }
        }

        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let mut community = tx.load_community().await?;
        community.update_details(
            &owner_id,
            CommunityName::new(name.clone()),
            description.clone().map(CommunityDescription::new),
            Utc::now(),
        )?;
        let events: Vec<MessagingEvent> = community.pull_events().into_iter().map(MessagingEvent::from).collect();

        if !tx.update_community_details(&name, description.as_deref()).await? {
            return Err(UpdateCommunityError::CommunityNotFound);
        }
        tx.save_events(&events, ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
