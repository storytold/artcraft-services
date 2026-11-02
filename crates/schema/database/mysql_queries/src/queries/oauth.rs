use sqlx::{Executor, MySql, Row};

pub struct InsertOauthClientArgs<'a, T> {
  pub token: &'a str,
  pub client_name: &'a str,
  pub owner_user_token: &'a str,
  pub mysql_executor: T,
}

pub async fn insert_oauth_client<'c, T>(args: InsertOauthClientArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(r#"INSERT INTO oauth_clients SET token = ?, client_name = ?, owner_user_token = ?"#).bind(args.token).bind(args.client_name).bind(args.owner_user_token).execute(args.mysql_executor).await?;
  Ok(())
}

pub struct InsertOauthRedirectArgs<'a, T> {
  pub client_token: &'a str,
  pub redirect_uri: &'a str,
  pub redirect_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn insert_oauth_redirect<'c, T>(args: InsertOauthRedirectArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(r#"INSERT INTO oauth_client_redirects SET client_token = ?, redirect_uri = ?, redirect_hash = ?"#).bind(args.client_token).bind(args.redirect_uri).bind(args.redirect_hash).execute(args.mysql_executor).await?;
  Ok(())
}

pub struct OauthClientRow {
  pub client_name: String,
}

pub struct GetOauthClientArgs<'a, T> {
  pub token: &'a str,
  pub mysql_executor: T,
}

pub async fn get_oauth_client<'c, T>(args: GetOauthClientArgs<'_, T>) -> Result<Option<OauthClientRow>, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let row = sqlx::query(r#"SELECT client_name FROM oauth_clients WHERE token = ? AND maybe_deleted_at IS NULL"#).bind(args.token).fetch_optional(args.mysql_executor).await?;
  Ok(row.map(|row| OauthClientRow { client_name: row.get("client_name") }))
}

pub struct RedirectMatchesArgs<'a, T> {
  pub client_token: &'a str,
  pub redirect_uri: &'a str,
  pub mysql_executor: T,
}

pub async fn redirect_matches<'c, T>(args: RedirectMatchesArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let row = sqlx::query(r#"SELECT id FROM oauth_client_redirects WHERE client_token = ? AND redirect_uri = ? LIMIT 1"#).bind(args.client_token).bind(args.redirect_uri).fetch_optional(args.mysql_executor).await?;
  Ok(row.is_some())
}

pub struct InsertNonceArgs<'a, T> {
  pub nonce_hash: &'a str,
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub request_hash: &'a str,
  pub ttl_seconds: i64,
  pub mysql_executor: T,
}

pub async fn insert_oauth_nonce<'c, T>(args: InsertNonceArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(
    r#"INSERT INTO oauth_consent_nonces
       SET nonce_hash = ?, user_token = ?, client_token = ?, request_hash = ?,
           expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP())"#,
  )
  .bind(args.nonce_hash)
  .bind(args.user_token)
  .bind(args.client_token)
  .bind(args.request_hash)
  .bind(args.ttl_seconds)
  .execute(args.mysql_executor)
  .await?;
  Ok(())
}

/// Marks one live nonce used. `true` only for the first caller with that hash and request.
pub struct ConsumeNonceArgs<'a, T> {
  pub nonce_hash: &'a str,
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub request_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn consume_oauth_nonce<'c, T>(args: ConsumeNonceArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let result = sqlx::query(
    r#"UPDATE oauth_consent_nonces
       SET maybe_used_at = UTC_TIMESTAMP()
       WHERE nonce_hash = ? AND user_token = ? AND client_token = ? AND request_hash = ?
         AND maybe_used_at IS NULL AND expires_at > UTC_TIMESTAMP()"#,
  )
  .bind(args.nonce_hash)
  .bind(args.user_token)
  .bind(args.client_token)
  .bind(args.request_hash)
  .execute(args.mysql_executor)
  .await?;
  Ok(result.rows_affected() == 1)
}

pub struct InsertCodeArgs<'a, T> {
  pub code_hash: &'a str,
  pub client_token: &'a str,
  pub user_token: &'a str,
  pub redirect_uri: &'a str,
  pub scopes: &'a str,
  pub code_challenge: &'a str,
  pub audience: &'a str,
  pub ttl_seconds: i64,
  pub mysql_executor: T,
}

pub async fn insert_oauth_code<'c, T>(args: InsertCodeArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(
    r#"INSERT INTO oauth_authorization_codes
       SET code_hash = ?, client_token = ?, user_token = ?, redirect_uri = ?, scopes = ?,
           code_challenge = ?, audience = ?, expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP())"#,
  )
  .bind(args.code_hash)
  .bind(args.client_token)
  .bind(args.user_token)
  .bind(args.redirect_uri)
  .bind(args.scopes)
  .bind(args.code_challenge)
  .bind(args.audience)
  .bind(args.ttl_seconds)
  .execute(args.mysql_executor)
  .await?;
  Ok(())
}

pub struct OauthCodeRow {
  pub client_token: String,
  pub user_token: String,
  pub redirect_uri: String,
  pub scopes: String,
  pub code_challenge: String,
  pub audience: String,
}

pub struct ConsumeCodeArgs<'a, T> {
  pub code_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn consume_oauth_code<'c, T>(args: ConsumeCodeArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let updated = sqlx::query(
    r#"UPDATE oauth_authorization_codes
       SET maybe_used_at = UTC_TIMESTAMP()
       WHERE code_hash = ? AND maybe_used_at IS NULL AND expires_at > UTC_TIMESTAMP()"#,
  )
  .bind(args.code_hash)
  .execute(args.mysql_executor)
  .await?;
  Ok(updated.rows_affected() == 1)
}

pub struct ReadCodeArgs<'a, T> {
  pub code_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn read_used_oauth_code<'c, T>(args: ReadCodeArgs<'_, T>) -> Result<Option<OauthCodeRow>, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let row = sqlx::query(
    r#"SELECT client_token, user_token, redirect_uri, scopes, code_challenge, audience
       FROM oauth_authorization_codes
       WHERE code_hash = ? AND maybe_used_at IS NOT NULL"#,
  )
  .bind(args.code_hash)
  .fetch_optional(args.mysql_executor)
  .await?;
  Ok(row.map(code_row))
}

fn code_row(row: sqlx::mysql::MySqlRow) -> OauthCodeRow {
  OauthCodeRow { client_token: row.get("client_token"), user_token: row.get("user_token"), redirect_uri: row.get("redirect_uri"), scopes: row.get("scopes"), code_challenge: row.get("code_challenge"), audience: row.get("audience") }
}

pub struct InsertTokenArgs<'a, T> {
  pub access_hash: &'a str,
  pub refresh_hash: &'a str,
  pub client_token: &'a str,
  pub user_token: &'a str,
  pub scopes: &'a str,
  pub audience: &'a str,
  pub access_ttl_seconds: i64,
  pub refresh_ttl_seconds: i64,
  pub mysql_executor: T,
}

pub async fn insert_oauth_token<'c, T>(args: InsertTokenArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(
    r#"INSERT INTO oauth_tokens
       SET access_hash = ?, refresh_hash = ?, client_token = ?, user_token = ?, scopes = ?, audience = ?,
           access_expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP()),
           refresh_expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP())"#,
  )
  .bind(args.access_hash)
  .bind(args.refresh_hash)
  .bind(args.client_token)
  .bind(args.user_token)
  .bind(args.scopes)
  .bind(args.audience)
  .bind(args.access_ttl_seconds)
  .bind(args.refresh_ttl_seconds)
  .execute(args.mysql_executor)
  .await?;
  Ok(())
}

pub struct OauthAccessRow {
  pub user_token: String,
  pub scopes: String,
  pub audience: String,
}

pub struct GetAccessArgs<'a, T> {
  pub access_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn get_live_oauth_access<'c, T>(args: GetAccessArgs<'_, T>) -> Result<Option<OauthAccessRow>, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let row = sqlx::query(
    r#"SELECT user_token, scopes, audience FROM oauth_tokens
       WHERE access_hash = ? AND maybe_revoked_at IS NULL AND access_expires_at > UTC_TIMESTAMP()"#,
  )
  .bind(args.access_hash)
  .fetch_optional(args.mysql_executor)
  .await?;
  Ok(row.map(|row| OauthAccessRow { user_token: row.get("user_token"), scopes: row.get("scopes"), audience: row.get("audience") }))
}

pub struct OauthRefreshRow {
  pub client_token: String,
  pub user_token: String,
  pub scopes: String,
  pub audience: String,
}

pub struct TakeRefreshArgs<'a, T> {
  pub refresh_hash: &'a str,
  pub mysql_executor: T,
}

/// Claims a live refresh token by moving it to the previous-hash column. A second presenter of
/// that same hash then hits [`revoke_reused_refresh`].
pub async fn take_oauth_refresh<'c, T>(args: TakeRefreshArgs<'_, T>) -> Result<Option<OauthRefreshRow>, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let row = sqlx::query(
    r#"SELECT client_token, user_token, scopes, audience FROM oauth_tokens
       WHERE refresh_hash = ? AND maybe_revoked_at IS NULL AND refresh_expires_at > UTC_TIMESTAMP()"#,
  )
  .bind(args.refresh_hash)
  .fetch_optional(args.mysql_executor)
  .await?;
  Ok(row.map(|row| OauthRefreshRow { client_token: row.get("client_token"), user_token: row.get("user_token"), scopes: row.get("scopes"), audience: row.get("audience") }))
}

pub struct RotateRefreshArgs<'a, T> {
  pub old_refresh_hash: &'a str,
  pub client_token: &'a str,
  pub new_access_hash: &'a str,
  pub new_refresh_hash: &'a str,
  pub access_ttl_seconds: i64,
  pub mysql_executor: T,
}

pub async fn rotate_oauth_refresh<'c, T>(args: RotateRefreshArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let result = sqlx::query(
    r#"UPDATE oauth_tokens
       SET maybe_previous_refresh_hash = refresh_hash,
           refresh_hash = ?,
           access_hash = ?,
           access_expires_at = TIMESTAMPADD(SECOND, ?, UTC_TIMESTAMP())
       WHERE refresh_hash = ? AND client_token = ?
         AND maybe_revoked_at IS NULL AND refresh_expires_at > UTC_TIMESTAMP()"#,
  )
  .bind(args.new_refresh_hash)
  .bind(args.new_access_hash)
  .bind(args.access_ttl_seconds)
  .bind(args.old_refresh_hash)
  .bind(args.client_token)
  .execute(args.mysql_executor)
  .await?;
  Ok(result.rows_affected() == 1)
}

pub struct ReusedRefreshArgs<'a, T> {
  pub previous_refresh_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn revoke_reused_refresh<'c, T>(args: ReusedRefreshArgs<'_, T>) -> Result<bool, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let result = sqlx::query(
    r#"UPDATE oauth_tokens
       SET maybe_revoked_at = UTC_TIMESTAMP()
       WHERE maybe_previous_refresh_hash = ? AND maybe_revoked_at IS NULL"#,
  )
  .bind(args.previous_refresh_hash)
  .execute(args.mysql_executor)
  .await?;
  Ok(result.rows_affected() == 1)
}

pub struct RevokePresentedArgs<'a, T> {
  pub token_hash: &'a str,
  pub mysql_executor: T,
}

pub async fn revoke_oauth_token_hash<'c, T>(args: RevokePresentedArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(
    r#"UPDATE oauth_tokens
       SET maybe_revoked_at = UTC_TIMESTAMP()
       WHERE maybe_revoked_at IS NULL AND (access_hash = ? OR refresh_hash = ?)"#,
  )
  .bind(args.token_hash)
  .bind(args.token_hash)
  .execute(args.mysql_executor)
  .await?;
  Ok(())
}

pub struct SaveConsentArgs<'a, T> {
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub scopes: &'a str,
  pub mysql_executor: T,
}

pub async fn save_oauth_consent<'c, T>(args: SaveConsentArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(
    r#"INSERT INTO oauth_consents SET user_token = ?, client_token = ?, scopes = ?
       ON DUPLICATE KEY UPDATE scopes = VALUES(scopes), maybe_revoked_at = NULL"#,
  )
  .bind(args.user_token)
  .bind(args.client_token)
  .bind(args.scopes)
  .execute(args.mysql_executor)
  .await?;
  Ok(())
}

pub struct OauthGrantRow {
  pub client_token: String,
  pub client_name: String,
  pub scopes: String,
}

pub struct ListGrantsArgs<'a, T> {
  pub user_token: &'a str,
  pub mysql_executor: T,
}

pub async fn list_oauth_grants<'c, T>(args: ListGrantsArgs<'_, T>) -> Result<Vec<OauthGrantRow>, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let rows = sqlx::query(
    r#"SELECT c.token AS client_token, c.client_name, s.scopes
       FROM oauth_consents s
       JOIN oauth_clients c ON c.token = s.client_token
       WHERE s.user_token = ? AND s.maybe_revoked_at IS NULL AND c.maybe_deleted_at IS NULL
       ORDER BY c.client_name"#,
  )
  .bind(args.user_token)
  .fetch_all(args.mysql_executor)
  .await?;
  Ok(rows.into_iter().map(|row| OauthGrantRow { client_token: row.get("client_token"), client_name: row.get("client_name"), scopes: row.get("scopes") }).collect())
}

pub struct RevokeGrantArgs<'a, T> {
  pub user_token: &'a str,
  pub client_token: &'a str,
  pub mysql_executor: T,
}

pub async fn revoke_oauth_grant<'c, T>(args: RevokeGrantArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(
    r#"UPDATE oauth_consents SET maybe_revoked_at = UTC_TIMESTAMP()
       WHERE user_token = ? AND client_token = ? AND maybe_revoked_at IS NULL"#,
  )
  .bind(args.user_token)
  .bind(args.client_token)
  .execute(args.mysql_executor)
  .await?;
  Ok(())
}

pub struct OwnerBannedArgs<'a, T> {
  pub user_token: &'a str,
  pub mysql_executor: T,
}

/// `Ok(None)` when the user row is gone. `Ok(Some(true))` when the account is banned.
pub async fn oauth_owner_is_banned<'c, T>(args: OwnerBannedArgs<'_, T>) -> Result<Option<bool>, sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  let row = sqlx::query(r#"SELECT is_banned FROM users WHERE token = ? LIMIT 1"#).bind(args.user_token).fetch_optional(args.mysql_executor).await?;
  let Some(row) = row else {
    return Ok(None);
  };
  let banned: i8 = row.try_get("is_banned")?;
  Ok(Some(banned != 0))
}

pub async fn revoke_oauth_tokens_for_client<'c, T>(args: RevokeGrantArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: 'c + Executor<'c, Database = MySql>,
{
  sqlx::query(
    r#"UPDATE oauth_tokens SET maybe_revoked_at = UTC_TIMESTAMP()
       WHERE user_token = ? AND client_token = ? AND maybe_revoked_at IS NULL"#,
  )
  .bind(args.user_token)
  .bind(args.client_token)
  .execute(args.mysql_executor)
  .await?;
  Ok(())
}
