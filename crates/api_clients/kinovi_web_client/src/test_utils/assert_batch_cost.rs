use crate::pricing::cost::kinovi_fractional_generation_cost::KinoviFractionalGenerationCost;
use crate::pricing::kinovi_pricing_tier::KinoviPricingTier;

const FLOAT_TOLERANCE: f64 = 1e-8;

/// Check scaling of credits and fractional cents, then independently compute
/// whole cents from the complete batch using integer arithmetic. Multiplying
/// a single video's already-rounded cents can overcharge a batch.
pub(crate) fn assert_batch_cost(
  single: KinoviFractionalGenerationCost,
  batched: KinoviFractionalGenerationCost,
  count: u64,
  tier: KinoviPricingTier,
  context: &str,
) {
  let expected_credit_hundredths = (single.kinovi_credits * 100.0).round() as u64 * count;
  let expected_credits = expected_credit_hundredths as f64 / 100.0;
  let expected_fractional_cents = single.usd_cents_fractional * count as f64;
  assert_eq!(batched.kinovi_credits, expected_credits, "{context}: credits");
  assert!(
    (batched.usd_cents_fractional - expected_fractional_cents).abs() < FLOAT_TOLERANCE,
    "{context}: fractional cents: {} != {expected_fractional_cents}",
    batched.usd_cents_fractional,
  );

  // Credit purchase rates expressed in hundredths of a credit per dollar.
  let credits_per_dollar_hundredths = match tier {
    KinoviPricingTier::Consumer => 19_298,
    KinoviPricingTier::Enterprise => 24_316,
  };
  let numerator = expected_credit_hundredths * 100;
  assert_eq!(batched.usd_cents_rounded_up, numerator.div_ceil(credits_per_dollar_hundredths), "{context}: rounded up");
  assert_eq!(batched.usd_cents_rounded_down, numerator / credits_per_dollar_hundredths, "{context}: rounded down");
}
