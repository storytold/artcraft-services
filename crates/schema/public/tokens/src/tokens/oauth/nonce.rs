use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// Single-use nonce embedded in the consent form.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct OauthNonceToken(pub String);

impl_string_token!(OauthNonceToken);
impl_mysql_token_from_row!(OauthNonceToken);
impl_crockford_generator!(OauthNonceToken, 44usize, TokenPrefix::OauthNonce, CrockfordLower);
