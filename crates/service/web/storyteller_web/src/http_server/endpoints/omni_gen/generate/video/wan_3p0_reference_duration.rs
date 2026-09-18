use crate::http_server::common_responses::common_web_error::CommonWebError;

/// Check measured video references before billing or submitting either Wan model.
/// Images and audio never enter this duration budget. Return the ceiling of the
/// combined duration for the router's local validation metadata.
pub(super) fn validate_wan_3p0_reference_duration(
  output_seconds: u16,
  durations_millis: &[Option<u64>],
) -> Result<u16, CommonWebError> {
  let mut total_millis = 0;
  for duration in durations_millis {
    let millis = duration.ok_or_else(|| CommonWebError::BadInputWithSimpleMessage(
      "Could not determine a Wan reference video's duration".to_owned(),
    ))?;
    if !(1_000..=15_000).contains(&millis) {
      return Err(CommonWebError::BadInputWithSimpleMessage(
        "Each Wan reference video must be between 1 and 15 seconds".to_owned(),
      ));
    }
    total_millis += millis;
    if total_millis > 15_000 {
      return Err(CommonWebError::BadInputWithSimpleMessage(
        "Wan reference videos must total at most 15 seconds".to_owned(),
      ));
    }
  }
  if total_millis + u64::from(output_seconds) * 1_000 > 30_000 {
    return Err(CommonWebError::BadInputWithSimpleMessage(
      "Wan reference video duration plus output duration must not exceed 30 seconds".to_owned(),
    ));
  }
  Ok(total_millis.div_ceil(1_000) as u16)
}

#[cfg(test)]
mod tests {
  use actix_web::{http::StatusCode, ResponseError};
  use super::*;

  #[test]
  fn exact_boundaries_and_fractional_totals_are_accepted() {
    assert_eq!(validate_wan_3p0_reference_duration(30, &[]).unwrap(), 0);
    assert_eq!(validate_wan_3p0_reference_duration(20, &[Some(10_000)]).unwrap(), 10);
    assert_eq!(validate_wan_3p0_reference_duration(15, &[Some(15_000)]).unwrap(), 15);
    // Rounding each file first would reject this valid 14.2s total.
    assert_eq!(validate_wan_3p0_reference_duration(15, &[Some(6_100), Some(8_100)]).unwrap(), 15);
    assert_eq!(validate_wan_3p0_reference_duration(27, &[Some(1_500), Some(1_500)]).unwrap(), 3);
  }

  #[test]
  fn invalid_video_durations_return_http_400() {
    for (output, references) in [
      (20, vec![Some(10_001)]),
      (30, vec![Some(1_000)]),
      (16, vec![Some(15_000)]),
      (2, vec![Some(8_000), Some(8_000)]),
      (2, vec![Some(999)]),
      (2, vec![Some(15_001)]),
      (2, vec![Some(u64::MAX)]),
      (5, vec![None]),
    ] {
      let error = validate_wan_3p0_reference_duration(output, &references).unwrap_err();
      assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
    }
  }
}
