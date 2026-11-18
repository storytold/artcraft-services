use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// Refresh token for an authorization-code grant. Callers store only a hash.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct OauthRefreshToken(pub String);

impl_string_token!(OauthRefreshToken);
impl_mysql_token_from_row!(OauthRefreshToken);
impl_crockford_generator!(OauthRefreshToken, 78usize, TokenPrefix::OauthRefresh, CrockfordLower);
