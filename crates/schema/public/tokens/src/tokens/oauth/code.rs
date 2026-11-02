use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// One-time authorization code. Callers store only a hash of this value.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct OauthCodeToken(pub String);

impl_string_token!(OauthCodeToken);
impl_mysql_token_from_row!(OauthCodeToken);
impl_crockford_generator!(OauthCodeToken, 75usize, TokenPrefix::OauthCode, CrockfordLower);
