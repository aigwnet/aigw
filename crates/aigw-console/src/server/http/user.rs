use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    server::http::{ApiContext, ApiData, ApiError, ApiResponseResult, auth::ExtractUser},
    service::{self, UserPassword, UserProfile, check_password, query_user},
};

pub(crate) struct User;

impl User {
    pub async fn profile(
        ExtractUser(user, _email): ExtractUser,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<UserProfile> {
        let user = user.ok_or(anyhow::anyhow!("user is empty"))?;
        let r = query_user(&context.database_client.rb, &user)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(r)))
    }

    pub async fn update_profile(
        Path(user): Path<String>,
        State(context): State<ApiContext>,
        Json(profile): Json<UserProfile>,
    ) -> ApiResponseResult<bool> {
        service::update_profile(&context.database_client.rb, &user, profile)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(None))
    }

    pub async fn update_password(
        Path(user): Path<String>,
        State(context): State<ApiContext>,
        Json(password): Json<UserPassword>,
    ) -> ApiResponseResult<bool> {
        let (b, _, _, _) = check_password(&context.database_client.rb, &user, &password.password)
            .await
            .map_err(ApiError::from)?;
        if b {
            service::update_password(&context.database_client.rb, &user, password)
                .await
                .map_err(ApiError::from)?;
            Ok(ApiData(None))
        } else {
            Err(ApiError::BasicError(anyhow::anyhow!(
                "Password is incorrect"
            )))
        }
    }
}
