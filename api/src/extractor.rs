use axum::{RequestPartsExt, async_trait, extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use kernel::model::{auth::AccessToken, id::UserId, role::Role, user::User};
use registry::AppRegistry;
use shared::error::{AppError, AppResult};

pub struct AuthorizedUser {
    pub access_token: AccessToken,
    pub user: User,
}

impl AuthorizedUser {
    pub fn id(&self) -> UserId {
        self.user.id
    }

    pub fn is_admin(&self) -> bool {
        self.user.role == Role::Admin
    }
}

#[async_trait]
impl FromRequestParts<AppRegistry> for AuthorizedUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, registry: &AppRegistry) -> AppResult<Self> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::UnathorizedError)?;
        let access_token = AccessToken(bearer.token().to_string());

        let user_id = registry
            .auth_repository()
            .fetch_user_id_from_token(&access_token)
            .await?
            .ok_or(AppError::UnathorizedError)?;

        let user = registry
            .user_repository()
            .find_current_user(user_id)
            .await?
            .ok_or(AppError::UnauthenticatedError)?;

        Ok(Self { access_token, user })
    }
}
