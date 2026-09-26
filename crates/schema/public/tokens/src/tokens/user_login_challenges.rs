use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::prefixes::TokenPrefix;

/// Non-secret identifier for a login challenge. This never grants access.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct UserLoginChallengeToken(pub String);

impl_string_token!(UserLoginChallengeToken);
impl_mysql_token_from_row!(UserLoginChallengeToken);
impl_crockford_generator!(UserLoginChallengeToken, 32usize, TokenPrefix::UserLoginChallenge, CrockfordLower);
