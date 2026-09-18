# Wan 3.0 pricing and September 23 promotion

Last verified: **2026-09-18**. Recheck after **2026-09-23** (starting September 24).

The code and tests intentionally use the promotional Wan 3.0 rates below.
Prices stay at these values until a reviewed code change; there is no automatic
date-based switch. Kinovi advertises 30% off until September 23 without specifying
the cutoff time or timezone. Confirm the live prices when updating.
[Kinovi promotion and pricing](https://kinovi.ai/models/wan3-text-to-video)

## Rates currently used

All values are **Kinovi credits per output second**. Each family's rates apply
to text-to-video, image-to-video, and reference-to-video. Reference media does
not add billed input seconds. Sources for regular Wan:
[text](https://kinovi.ai/models/wan3-text-to-video),
[image](https://kinovi.ai/models/wan3-image-to-video),
[reference](https://kinovi.ai/models/wan3-ref-to-video).

| Family        | Resolution | Credits/second | Current status                 |
|---------------|------------|----------------|--------------------------------|
| Wan 3.0       | 480p       | 9.98           | Promotional until Sep 23       |
| Wan 3.0       | 720p       | 19.96          | Promotional until Sep 23       |
| Wan 3.0       | 1080p      | 39.91          | Promotional until Sep 23       |
| Wan 3.0 Prime | 480p       | 18.86          | No advertised Sep 23 promotion |
| Wan 3.0 Prime | 720p       | 38.83          | No advertised Sep 23 promotion |
| Wan 3.0 Prime | 1080p      | 77.66          | No advertised Sep 23 promotion |

Prime sources:
[text](https://kinovi.ai/models/wan3-prime-text-to-video),
[image](https://kinovi.ai/models/wan3-prime-image-to-video),
[reference](https://kinovi.ai/models/wan3-prime-ref-to-video).
Prime 480p for 30 seconds is **565.8 credits**; the original supplied reference
example's 568.8 was a typo. Recheck Prime independently; the regular Wan
promotion ending does not establish a Prime price change.

The promotional rates already include the discount. Total credits are the
selected rate multiplied by output seconds. Both pricing tiers currently use
the same credit rates; only their credit purchase rates differ. The client uses
192.98 credits/USD for Consumer and 243.16 for Enterprise, defined in
[pricing constants](src/pricing/constants.rs).
[KinoviPricingTier::cost_from_credits](src/pricing/kinovi_pricing_tier.rs) rounds
the total to hundredths of a credit and calculates fractional, floor, and ceiling
USD cents. Use those purchase rates for client costs; Kinovi's displayed USD
prices use a different conversion.

## Advertised non-promotional rates to recheck

These are the crossed-out prices shown alongside the promotion on September 18.
They are **reference values for the later update**, not active calculator rates
or a guarantee of what Kinovi will charge after the promotion.
The same values appear on the regular Wan
[text](https://kinovi.ai/models/wan3-text-to-video),
[image](https://kinovi.ai/models/wan3-image-to-video), and
[reference](https://kinovi.ai/models/wan3-ref-to-video) pricing pages.

| Resolution | Current promotional credits/second | Advertised non-promotional credits/second |
|------------|------------------------------------|-------------------------------------------|
| 480p       | 9.98                               | 14.2571                                   |
| 720p       | 19.96                              | 28.5143                                   |
| 1080p      | 39.91                              | 57.0143                                   |

## How to update after September 23

1. Reopen the six model pages above and confirm the promotion ended, the exact
   credit rates, and total-credit rounding. Check consumer quotes for multiple
   durations and every resolution; the web client uses the consumer workflow.
   Record the verification date and evidence in this document.
2. Update `WAN_CREDIT_HUNDREDTHS_PER_SECOND` and `Wan3p0Model::calculate_costs`
   in [wan_3p0.rs](src/generate/video/wan_3p0.rs). The current table is
   `[998, 1996, 3991]` in 480p/720p/1080p order. **The advertised replacement
   rates have four decimal places:** the existing hundredths representation
   cannot preserve them. If confirmed, use a sufficient fixed-point scale
   (for example 10,000), rename the constants to match their units, and adjust
   the final divisor. Preserve rate precision until after multiplying by
   duration; verify provider rounding before changing the shared credit
   conversion. If both families change scale, rescale the Prime integers while
   preserving Prime's monetary rates unless a separate price change is confirmed.
3. Update the literal expectations in
   [wan_3p0_tests.rs](src/generate/video/wan_3p0_tests.rs): `WAN_CREDITS`,
   `WAN_CENTS`, the `199.6` regular `reference_media_tests!` argument (720p/10s),
   and the `99.8` regular image `check!` argument (720p/5s). The default-price
   and seed/audio tests already read the literal tables. Keep expected prices
   independent of production rate constants and assert both pricing tiers.
   Update Prime expectations only if its verified monetary rates change.
4. Update this document and the dated comments beside the production constants
   and test tables. Keep the promotional snapshot as historical context.
5. Run from the repository root:

   ```sh
   cargo test --offline -p kinovi_web_client --lib wan_3p0_tests
   cargo test --offline -p kinovi_web_client --lib
   ```

   The Wan HTTP tests use a local server and need permission to bind localhost.
   These commands leave the existing ignored live-generation tests ignored.
