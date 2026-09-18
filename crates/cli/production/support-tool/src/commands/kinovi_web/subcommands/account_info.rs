use anyhow::anyhow;
use clap::Args;
use kinovi_web_client::creds::kinovi_web_session::KinoviWebSession;
use kinovi_web_client::requests::get_user_auth_details::get_user_auth_details::{
  get_user_auth_details, GetUserAuthDetailsArgs,
};

use super::super::state::KinoviWebState;

/// Print the account's current credit balances (auth.user endpoint) as a
/// single CSV line: account,email,credits,available_credits.
#[derive(Args)]
pub struct AccountInfoArgs {
  /// Account label for the output line. Defaults to the cookies env var name.
  #[arg(long)]
  pub account: Option<String>,
}

pub async fn run(state: &KinoviWebState, args: AccountInfoArgs) -> anyhow::Result<()> {
  let account_label = args.account.as_deref().unwrap_or(&state.cookies_env);
  let session = KinoviWebSession::from_cookies_string(state.cookies.clone());

  let details = get_user_auth_details(GetUserAuthDetailsArgs {
    session: &session,
    host_override: None,
  }).await
    .map_err(|err| anyhow!("Error fetching auth details: {:?}", err))?;

  println!("{},{},{},{}",
    account_label, details.email, details.credits, details.available_credits);
  Ok(())
}
