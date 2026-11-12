use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// Access token for an authorization-code grant. Callers store only a hash.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct OauthAccessToken(pub String);

impl_string_token!(OauthAccessToken);
impl_mysql_token_from_row!(OauthAccessToken);
impl_crockford_generator!(OauthAccessToken, 77usize, TokenPrefix::OauthAccess, CrockfordLower);
