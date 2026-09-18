use clap::{Args, Subcommand};

use super::account::KinoviAccount;
use super::state::KinoviWebState;
use super::subcommands;

/// All canonical subcommand names for this module.
/// Used by the underscore-insensitive arg normalizer.
pub const SUBCOMMAND_NAMES: &[&str] = &[
  "account_info",
  "audit_credits",
  "audit_orders",
  "audit_payments",
  "failed_job_histogram",
  "find_job",
  "generate_video",
];

#[derive(Args)]
pub struct KinoviWebArgs {
  /// Kinovi account whose cookie configuration to use.
  #[arg(long, global = true, value_enum, ignore_case = true, default_value = "volcengine")]
  pub kinovi_account: KinoviAccount,

  /// Read cookies from this env var, overriding --kinovi-account only when supplied.
  #[arg(long = "cookies-env", global = true, value_name = "ENV_VAR_NAME")]
  pub maybe_cookies_env: Option<String>,

  #[command(subcommand)]
  pub command: KinoviWebCommand,
}

#[derive(Subcommand)]
#[command(rename_all = "snake_case")]
pub enum KinoviWebCommand {
  /// Print the account's current credit balances as one CSV line
  AccountInfo(subcommands::account_info::AccountInfoArgs),

  /// Dump the account's credits ledger (credits.getCreditHistory) to CSV
  /// back to a start date, for refund-entry auditing
  AuditCredits(subcommands::audit_credits::AuditCreditsArgs),

  /// Dump Kinovi's per-order billing records (totalCredits) to CSV for a
  /// date window, to audit whether failed orders are charged
  AuditOrders(subcommands::audit_orders::AuditOrdersArgs),

  /// Dump the account's billing payments history (credit-package
  /// purchases) to CSV, for reconciling refills against invoices
  AuditPayments(subcommands::audit_payments::AuditPaymentsArgs),

  /// Find a job by its order ID across all pages
  FindJob(subcommands::find_job::FindJobArgs),

  /// Scan all jobs and print a histogram of failure reasons
  FailedJobHistogram,

  /// Generate a video via KinoviWeb/Kinovi directly
  GenerateVideo(subcommands::generate_video::GenerateVideoArgs),
}

pub async fn run(args: KinoviWebArgs) -> anyhow::Result<()> {
  let state = KinoviWebState::from_env(args.kinovi_account, args.maybe_cookies_env.as_deref())?;

  match args.command {
    KinoviWebCommand::AccountInfo(args) => subcommands::account_info::run(&state, args).await,
    KinoviWebCommand::AuditCredits(args) => subcommands::audit_credits::run(&state, args).await,
    KinoviWebCommand::AuditOrders(args) => subcommands::audit_orders::run(&state, args).await,
    KinoviWebCommand::AuditPayments(args) => subcommands::audit_payments::run(&state, args).await,
    KinoviWebCommand::FindJob(args) => subcommands::find_job::run(&state, args).await,
    KinoviWebCommand::FailedJobHistogram => subcommands::failed_job_histogram::run(&state).await,
    KinoviWebCommand::GenerateVideo(args) => subcommands::generate_video::run(&state, args).await,
  }
}

#[cfg(test)]
mod tests {
  use clap::{error::ErrorKind, Parser};

  use super::*;
  use crate::commands::run::{all_canonical_names, Cli, TopLevelCommand};
  use crate::utils::normalize_subcommands::normalize_subcommand_args;

  const COMMANDS: &[&[&str]] = &[
    &["account_info"],
    &["audit_credits", "--out", "/tmp/credits.csv"],
    &["audit_orders", "--out", "/tmp/orders.csv"],
    &["audit_payments", "--out", "/tmp/payments.csv"],
    &["find_job", "--token", "test-order"],
    &["failed_job_histogram"],
    &["generate_video", "--prompt", "A corgi at the lake"],
  ];

  mod cli_parsing {
    use super::*;

    #[test]
    fn every_command_defaults_to_volcengine_without_a_cookie_override() {
      for command in COMMANDS {
        let args = parse_kinovi(command);
        assert_eq!(args.kinovi_account, KinoviAccount::Volcengine);
        assert!(args.maybe_cookies_env.is_none());
      }
    }

    #[test]
    fn every_command_accepts_all_accounts_and_cookie_overrides_before_or_after_subcommand() {
      for (value, account) in [
        ("volcengine", KinoviAccount::Volcengine),
        ("byteplus", KinoviAccount::BytePlus),
        ("byteplusultra", KinoviAccount::BytePlusUltra),
      ] {
        for command in COMMANDS {
          let flags = ["--kinovi-account", value, "--cookies-env", "CUSTOM_COOKIES"];
          for args in [
            [flags.as_slice(), *command].concat(),
            [*command, flags.as_slice()].concat(),
          ] {
            let args = parse_kinovi(&args);
            assert_eq!(args.kinovi_account, account);
            assert_eq!(args.maybe_cookies_env.as_deref(), Some("CUSTOM_COOKIES"));
          }
        }
      }
    }

    #[test]
    fn aliases_and_underscore_normalization_preserve_account_selection() {
      for alias in ["kinovi_web", "kinoviweb", "kinovi", "seedance2pro", "seedance2_pro"] {
        let args = ["support-tool", alias, "accountinfo", "--kinovi-account", "BYTEPLUSULTRA"];
        let args = normalize_subcommand_args(args.map(String::from), &all_canonical_names());
        let cli = Cli::try_parse_from(args).unwrap();
        let TopLevelCommand::KinoviWeb(args) = cli.command else {
          panic!("expected Kinovi command");
        };
        assert_eq!(args.kinovi_account, KinoviAccount::BytePlusUltra);
      }
    }

    #[test]
    fn rejects_unknown_accounts() {
      let err = Cli::try_parse_from([
        "support-tool", "kinovi_web", "account_info", "--kinovi-account", "unknown",
      ]).err().expect("unknown accounts should be rejected");
      assert_eq!(err.kind(), ErrorKind::InvalidValue);
    }
  }

  fn parse_kinovi(args: &[&str]) -> KinoviWebArgs {
    let cli = Cli::try_parse_from([&["support-tool", "kinovi_web"], args].concat()).unwrap();
    match cli.command {
      TopLevelCommand::KinoviWeb(args) => args,
      _ => panic!("expected Kinovi command"),
    }
  }
}
