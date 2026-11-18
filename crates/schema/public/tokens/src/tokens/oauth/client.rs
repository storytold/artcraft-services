use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// Public client id for an OAuth client. This value is not a secret.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct OauthClientToken(pub String);

impl_string_token!(OauthClientToken);
impl_mysql_token_from_row!(OauthClientToken);
impl_crockford_generator!(OauthClientToken, 45usize, TokenPrefix::OauthClient, CrockfordLower);
