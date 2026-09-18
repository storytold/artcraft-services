use anyhow::anyhow;

use super::account::KinoviAccount;

pub struct KinoviWebState {
  pub cookies: String,
  pub cookies_env: String,
}

impl KinoviWebState {
  pub fn from_env(account: KinoviAccount, maybe_cookies_env: Option<&str>) -> anyhow::Result<Self> {
    Self::from_env_lookup(account, maybe_cookies_env, easyenv::get_env_string_optional)
  }

  fn from_env_lookup(
    account: KinoviAccount,
    maybe_cookies_env: Option<&str>,
    mut lookup: impl FnMut(&str) -> Option<String>,
  ) -> anyhow::Result<Self> {
    // An explicit override must resolve on its own, without requiring or falling
    // back to credentials for the selected account.
    let override_vars;
    let cookie_env_vars = match maybe_cookies_env {
      Some(var) => {
        override_vars = [var];
        &override_vars[..]
      },
      None => account.cookie_env_vars(),
    };

    for &var in cookie_env_vars {
      if let Some(cookies) = lookup(var) {
        return Ok(Self {
          cookies,
          cookies_env: var.to_string(),
        });
      }
    }

    Err(anyhow!("Missing Kinovi cookies: set {}", cookie_env_vars.join(" or ")))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const COOKIES: &[(&str, &str)] = &[
    ("SEEDANCE2PRO_VOLCENGINE_COOKIES", "volcengine-cookies"),
    ("SEEDANCE2PRO_BYTEPLUS_COOKIES", "byteplus-cookies"),
    ("SEEDANCE2PRO_BYTEPLUS_ULTRA_COOKIES", "ultra-cookies"),
    ("SEEDANCE2PRO_COOKIES", "legacy-cookies"),
    ("SEEDANCE2PRO_ALT_COOKIES", "legacy-alt-cookies"),
    ("CUSTOM_COOKIES", "override-cookies"),
  ];

  mod account_selection {
    use super::*;

    #[test]
    fn selects_each_account_even_when_legacy_cookies_are_set() {
      for (account, var, cookies) in [
        (KinoviAccount::Volcengine, "SEEDANCE2PRO_VOLCENGINE_COOKIES", "volcengine-cookies"),
        (KinoviAccount::BytePlus, "SEEDANCE2PRO_BYTEPLUS_COOKIES", "byteplus-cookies"),
        (KinoviAccount::BytePlusUltra, "SEEDANCE2PRO_BYTEPLUS_ULTRA_COOKIES", "ultra-cookies"),
      ] {
        for env in [COOKIES, &[(var, cookies)]] {
          let state = resolve(account, None, env).unwrap();
          assert_eq!(state.cookies, cookies);
          assert_eq!(state.cookies_env, var);
        }
      }
    }

    #[test]
    fn falls_back_to_the_matching_legacy_variable() {
      let env = &COOKIES[3..5];
      for (account, var, cookies) in [
        (KinoviAccount::Volcengine, "SEEDANCE2PRO_COOKIES", "legacy-cookies"),
        (KinoviAccount::BytePlus, "SEEDANCE2PRO_ALT_COOKIES", "legacy-alt-cookies"),
      ] {
        let state = resolve(account, None, env).unwrap();
        assert_eq!(state.cookies, cookies);
        assert_eq!(state.cookies_env, var);
      }
    }

    #[test]
    fn missing_account_cookies_never_use_another_accounts_credentials() {
      for (account, expected_var) in [
        (KinoviAccount::BytePlus, "SEEDANCE2PRO_BYTEPLUS_COOKIES"),
        (KinoviAccount::BytePlusUltra, "SEEDANCE2PRO_BYTEPLUS_ULTRA_COOKIES"),
      ] {
        let err = resolve(account, None, &COOKIES[3..4]).err().expect("account cookies are missing");
        assert!(err.to_string().contains(expected_var));
      }
    }
  }

  mod cookie_overrides {
    use super::*;

    #[test]
    fn explicit_override_wins_with_or_without_account_cookies() {
      for account in [KinoviAccount::Volcengine, KinoviAccount::BytePlus, KinoviAccount::BytePlusUltra] {
        for env in [COOKIES, &COOKIES[5..]] {
          let state = resolve(account, Some("CUSTOM_COOKIES"), env).unwrap();
          assert_eq!(state.cookies, "override-cookies");
          assert_eq!(state.cookies_env, "CUSTOM_COOKIES");
        }
      }
    }

    #[test]
    fn missing_explicit_override_errors_instead_of_using_account_cookies() {
      let err = resolve(KinoviAccount::Volcengine, Some("MISSING_COOKIES"), COOKIES)
        .err().expect("explicit override must exist");
      assert_eq!(err.to_string(), "Missing Kinovi cookies: set MISSING_COOKIES");
    }
  }

  fn resolve(
    account: KinoviAccount,
    maybe_override: Option<&str>,
    env: &[(&str, &str)],
  ) -> anyhow::Result<KinoviWebState> {
    KinoviWebState::from_env_lookup(account, maybe_override, |var| {
      env.iter().find(|(name, _)| *name == var).map(|(_, value)| value.to_string())
    })
  }
}
