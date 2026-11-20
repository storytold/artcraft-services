use crate::http_server::common_responses::common_web_error::CommonWebError;
use crate::http_server::endpoints::users::google_sso::handle_new_sso_account_for_existing_user::{handle_new_sso_account_for_existing_user, LinkArgs};
use crate::http_server::endpoints::users::google_sso::handle_new_sso_account_for_new_user::{handle_new_sso_account_for_new_user, CreateArgs};
use crate::util::canonicalize_email_for_users_table::canonicalize_email_for_users_table;
use actix_web::HttpRequest;
use google_sign_in::claims::claims::Claims;
use log::warn;
use mysql_queries::queries::users::user::get::get_user_email_confirmation::{get_user_email_confirmation, GetUserEmailConfirmationArgs, UserEmailConfirmation};
use mysql_queries::queries::users::user::get::lookup_user_for_login_by_email_with_transactor::lookup_user_for_login_by_email_with_transactor;
use mysql_queries::queries::users::user::get::lookup_user_for_login_result::UserRecordForLogin;
use mysql_queries::utils::transactor::Transactor;
use sqlx::pool::PoolConnection;
use sqlx::MySql;
use tokens::tokens::users::UserToken;

const EXISTING_ACCOUNT_MESSAGE: &str = "An account with a password already uses this email address. \
  Sign in with the password, or reset it to confirm the address; Google sign-in then works for it.";

pub struct NewSsoArgs<'a> {
  pub http_request: &'a HttpRequest,
  pub claims: Claims,
  pub claims_subject: &'a str,
  pub claims_email_address: &'a str,
  pub mysql_connection: &'a mut PoolConnection<MySql>,
  pub maybe_referral_url: Option<String>,
  pub maybe_landing_url: Option<String>,
  pub maybe_referral_partner: Option<String>,
  pub maybe_referral_user_token: Option<UserToken>,
}

pub struct NewSsoAccountInfo {
  pub user_token: UserToken,
  pub user_display_name: String,
  pub username_is_not_customized: bool,
  pub is_new_account: bool,
}

pub async fn handle_new_sso_account(
  args: NewSsoArgs<'_>
)
  -> Result<NewSsoAccountInfo, CommonWebError>
{
  // NB: We use this routine to "normalize" email addresses in the user table.
  // It won't necessarily match the email address in the Google claims.
  // A better normalization function in the future may handle dots and plus
  // signs in Gmail addresses, for instance.
  let user_email_address = canonicalize_email_for_users_table(&args.claims_email_address);

  let maybe_user_account =
      lookup_user_for_login_by_email_with_transactor(&user_email_address, Transactor::for_connection(args.mysql_connection))
          .await
          .map_err(|err| {
            warn!("error looking up user by email: {:?}", err);
            CommonWebError::from_anyhow_error(err)
          })?;

  match maybe_user_account {
    Some(user_account) => {
      if !may_link_to_existing_account(args.claims.email_verified(), &user_account, args.mysql_connection).await? {
        return Err(CommonWebError::BadInputWithSimpleMessage(EXISTING_ACCOUNT_MESSAGE.to_string()));
      }

      handle_new_sso_account_for_existing_user(LinkArgs {
        http_request: args.http_request,
        claims: args.claims,
        claims_subject: args.claims_subject,
        claims_email_address: &args.claims_email_address,
        user_account,
        mysql_connection: args.mysql_connection,
      }).await
    },
    None => {
      handle_new_sso_account_for_new_user(CreateArgs {
        http_request: args.http_request,
        claims: args.claims,
        claims_subject: args.claims_subject,
        claims_email_address: &args.claims_email_address,
        user_email_address: &user_email_address,
        mysql_connection: args.mysql_connection,
        maybe_referral_url: args.maybe_referral_url,
        maybe_landing_url: args.maybe_landing_url,
        maybe_referral_partner: args.maybe_referral_partner,
        maybe_referral_user_token: args.maybe_referral_user_token,
      }).await
    },
  }
}

/// Whether a Google identity may be attached to an existing account found by its email address.
///
/// Google has to vouch for the address. An account that can also be signed into with a password
/// has to have shown the address is its own too (by us, or by Google when it was created);
/// otherwise whoever registered the address first, without owning it, would share the account
/// with the address's real owner.
async fn may_link_to_existing_account(
  google_email_verified: bool,
  user_account: &UserRecordForLogin,
  mysql_connection: &mut PoolConnection<MySql>,
) -> Result<bool, CommonWebError> {
  if !google_email_verified {
    return Ok(false);
  }
  let maybe_confirmation = get_user_email_confirmation(GetUserEmailConfirmationArgs {
    user_token: &user_account.token,
    mysql_executor: &mut **mysql_connection,
  })
      .await
      .map_err(|err| {
        warn!("error looking up email confirmation: {:?}", err);
        CommonWebError::from_error(err)
      })?;
  Ok(maybe_confirmation.is_some_and(confirmation_allows_link))
}

/// No password to protect (`users.is_without_password`), or the address is confirmed.
///
/// The flag, not `password_hash`: that column is `BINARY(60)`, so the `*` stored for an account
/// without a password comes back padded with NUL bytes.
fn confirmation_allows_link(confirmation: UserEmailConfirmation) -> bool {
  confirmation.is_without_password || confirmation.email_confirmed || confirmation.email_confirmed_by_google
}

#[cfg(test)]
mod tests {
  use mysql_queries::queries::users::user::create::create_account_from_stripe_checkout::{create_account_from_stripe_checkout, CreateAccountFromStripeCheckoutArgs};
  use mysql_queries::queries::users::user::get::get_user_email_confirmation::UserEmailConfirmation;
  use mysql_queries::queries::users::user::get::lookup_user_for_login_by_email_with_transactor::lookup_user_for_login_by_email_with_transactor;
  use mysql_queries::queries::users::user::get::lookup_user_for_login_result::UserRecordForLogin;
  use mysql_queries::utils::transactor::Transactor;
  use mysql_testing::pool::create_test_pool;
  use sqlx::pool::PoolConnection;
  use sqlx::MySql;
  use tokens::tokens::users::UserToken;

  use super::{confirmation_allows_link, may_link_to_existing_account};

  const UNCONFIRMED: UserEmailConfirmation = UserEmailConfirmation { email_confirmed: false, email_confirmed_by_google: false, is_without_password: false };
  const CONFIRMED: UserEmailConfirmation = UserEmailConfirmation { email_confirmed: true, email_confirmed_by_google: false, is_without_password: false };
  const CONFIRMED_BY_GOOGLE: UserEmailConfirmation = UserEmailConfirmation { email_confirmed: false, email_confirmed_by_google: true, is_without_password: false };

  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn an_unverified_google_email_never_links() {
    let (user_account, mut connection) = checkout_account_looked_up_by_email().await;
    assert!(!may_link_to_existing_account(false, &user_account, &mut connection).await.unwrap());
  }

  #[test]
  fn a_password_account_links_only_once_its_email_is_confirmed() {
    assert!(!confirmation_allows_link(UNCONFIRMED));
    assert!(confirmation_allows_link(CONFIRMED));
    assert!(confirmation_allows_link(CONFIRMED_BY_GOOGLE));
  }

  /// Goes through the real `users` row: checkout stores `*` in the `BINARY(60)` hash column.
  #[tokio::test]
  #[cfg_attr(feature = "skip_database_tests", ignore)]
  async fn an_account_without_a_password_links_as_before() {
    let (user_account, mut connection) = checkout_account_looked_up_by_email().await;
    assert_eq!(user_account.password_hash.len(), 60, "the column pads the stored `*`");
    assert!(
      may_link_to_existing_account(true, &user_account, &mut connection).await.unwrap(),
      "an account made by checkout has no password, so a verified Google email links to it"
    );
  }

  /// An account made the way checkout makes one, then looked up by email as the handler does.
  async fn checkout_account_looked_up_by_email() -> (UserRecordForLogin, PoolConnection<MySql>) {
    let pool = create_test_pool().await;
    let suffix = UserToken::generate().as_str().chars().rev().take(10).collect::<String>().to_lowercase();
    let username = format!("checkout_{suffix}");
    let email_address = format!("{username}@example-test.invalid");

    create_account_from_stripe_checkout(CreateAccountFromStripeCheckoutArgs {
      username: &username,
      display_name: &username,
      email_address: &email_address,
      email_gravatar_hash: "00000000000000000000000000000000",
      ip_address: "127.0.0.1",
      maybe_source: None,
      maybe_referral_url: None,
      maybe_landing_url: None,
      maybe_feature_flags: None,
      maybe_referral_partner: None,
      maybe_referral_user_token: None,
    }, &pool).await.unwrap();

    let mut connection = pool.acquire().await.unwrap();
    let user_account = lookup_user_for_login_by_email_with_transactor(&email_address, Transactor::for_connection(&mut connection))
        .await
        .unwrap()
        .expect("the account just created");
    (user_account, connection)
  }
}
