use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use errors::AnyhowResult;
use ffprobe::FfProbe;
use tempfile::NamedTempFile;

#[derive(Debug, Default)]
pub struct VideoInfo {
  pub dimensions: Option<VideoDimensions>,
  pub duration: Option<VideoDuration>,
  pub frame_rate: Option<VideoFrameRate>,
}

#[derive(Debug)]
pub struct VideoDimensions {
  pub width: u64,
  pub height: u64,
}

#[derive(Debug)]
pub struct VideoDuration {
  /// We have to convert ffprobe's seconds (with decimal) into milliseconds.
  /// We use a u32 as this can hold 49 days of milliseconds.
  pub millis: u32,

  /// This is the original value returned by ffprobe (for debugging).
  pub seconds_original: String,
}

#[derive(Debug)]
pub struct VideoFrameRate {
  pub fps: f32,
  /// This is the original value returned by ffprobe (for debugging).
  pub fps_original: String,
}

pub fn ffprobe_get_info(
  video_path: impl AsRef<Path>
) -> AnyhowResult<VideoInfo>
{
  let result = ffprobe::ffprobe(video_path)?;
  Ok(info_from_probe(result))
}

/// Probe downloaded/uploaded bytes without changing their container or audio.
/// A seekable temporary file lets ffprobe inspect MOV metadata at the end of the file.
pub fn ffprobe_get_info_from_bytes(bytes: &[u8]) -> AnyhowResult<VideoInfo> {
  let mut file = NamedTempFile::new()?;
  file.write_all(bytes)?;
  ffprobe_get_info(file.path())
}

fn info_from_probe(result: FfProbe) -> VideoInfo {
  let Some(stream) = result.streams.iter().find(|stream| {
    stream.codec_type.as_deref() == Some("video") && stream.disposition.attached_pic == 0
  }) else {
    return VideoInfo::default();
  };

  // MOV files can carry duration only at the container level. Keep dimensions
  // even when the stream-level duration is absent.
  let dimensions = stream.width.zip(stream.height)
    .filter(|(width, height)| *width > 0 && *height > 0)
    .map(|(width, height)| VideoDimensions { width: width as u64, height: height as u64 });
  let duration = [stream.duration.as_deref(), result.format.duration.as_deref()]
    .into_iter().flatten()
    .find_map(|seconds| parse_seconds(seconds).ok().map(|millis| VideoDuration {
      millis,
      seconds_original: seconds.to_string(),
    }));
  let frame_rate = [&stream.avg_frame_rate, &stream.r_frame_rate].into_iter()
    .find_map(|rate| parse_fps(rate).ok()
      .filter(|fps| fps.is_finite() && *fps > 0.0)
      .map(|fps| VideoFrameRate { fps, fps_original: rate.to_string() }));

  VideoInfo { dimensions, duration, frame_rate }
}

fn parse_seconds(ffprobe_seconds: &str) -> AnyhowResult<u32> {
  let (seconds, decimal_seconds) = ffprobe_seconds.split_once('.')
      .unwrap_or_else(|| (ffprobe_seconds, ""));

  let seconds = u32::from_str(seconds)?;
  let milliseconds = seconds.saturating_mul(1000);

  let decimal_seconds = f32::from_str(&format!("0.{decimal_seconds}"))?;
  let remaining_millis = (decimal_seconds * 1000.0).round() as u32;

  let total_milliseconds = milliseconds.saturating_add(remaining_millis);
  Ok(total_milliseconds)
}

fn parse_fps(ffprobe_fps: &str) -> AnyhowResult<f32> {
  if let Some((num, denom)) = ffprobe_fps.split_once('/') {
    let num = f32::from_str(num)?;
    let denom = f32::from_str(denom)?;
    let fps = num / denom;
    return Ok(fps);
  }
  let fps = f32::from_str(ffprobe_fps)?;
  Ok(fps)
}

#[cfg(test)]
pub mod tests {
  use test_utils::test_file_path::test_file_path;

  use super::{ffprobe_get_info, parse_seconds};

  #[test]
  pub fn test_decode_mp4() {
    let filename = test_file_path("test_data/video/mp4/golden_sun_garoh.mp4")
        .expect("path should exist");

    let info = ffprobe_get_info(filename)
        .expect("should be able to read with ffprobe");

    let dimensions = info.dimensions.expect("video should have dimensions");

    assert_eq!(dimensions.width, 640);
    assert_eq!(dimensions.height, 480);
    assert_eq!(info.duration.unwrap().millis, 15133);
    assert_eq!(info.frame_rate.unwrap().fps, 30.0);
  }

  mod quicktime {
    use super::super::{ffprobe_get_info_from_bytes, info_from_probe};
    use ffprobe::{FfProbe, Format, Stream};
    use std::process::Command;
    use tempdir::TempDir;

    #[test]
    fn container_duration_does_not_discard_dimensions() {
      let info = info_from_probe(FfProbe {
        streams: vec![Stream {
          codec_type: Some("video".into()),
          width: Some(320),
          height: Some(180),
          duration: None,
          avg_frame_rate: "0/0".into(),
          r_frame_rate: "24/1".into(),
          ..Default::default()
        }],
        format: Format { duration: Some("1.250".into()), ..Default::default() },
      });
      let dimensions = info.dimensions.unwrap();
      assert_eq!((dimensions.width, dimensions.height), (320, 180));
      assert_eq!(info.duration.unwrap().millis, 1250);
      assert_eq!(info.frame_rate.unwrap().fps, 24.0);
    }

    #[test]
    fn probes_mov_with_pcm_audio_without_modifying_bytes() {
      let dir = TempDir::new("quicktime-metadata-test").unwrap();
      let path = dir.path().join("reference.mov");
      let output = Command::new("ffmpeg").args([
        "-hide_banner", "-loglevel", "error", "-nostdin",
        "-f", "lavfi", "-i", "color=c=blue:s=160x90:r=24",
        "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000",
        "-t", "1", "-c:v", "libx264", "-pix_fmt", "yuv420p",
        "-c:a", "pcm_s16le", "-ac", "2",
      ]).arg(&path).output().unwrap();
      assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
      let bytes = std::fs::read(&path).unwrap();
      let info = ffprobe_get_info_from_bytes(&bytes).unwrap();
      let dimensions = info.dimensions.unwrap();
      assert_eq!((dimensions.width, dimensions.height), (160, 90));
      assert_eq!(info.duration.unwrap().millis, 1000);
      assert_eq!(info.frame_rate.unwrap().fps, 24.0);
      assert_eq!(std::fs::read(&path).unwrap(), bytes);
      let probe = ffprobe::ffprobe(&path).unwrap();
      assert!(probe.streams.iter().any(|s| s.codec_name.as_deref() == Some("pcm_s16le")));
    }
  }

  mod parse_seconds {
    use super::*;

    #[test]
    pub fn one_second() {
      let seconds = "1.000";
      let millis = parse_seconds(seconds).expect("should be able to parse seconds");
      assert_eq!(millis, 1000);
    }

    #[test]
    pub fn seconds_no_decimal() {
      // NB: I'm not sure if ffprobe returns data like this. Just covering all bases.
      let seconds = "5.";
      let millis = parse_seconds(seconds).expect("should be able to parse seconds");
      assert_eq!(millis, 5000);
    }

    #[test]
    pub fn seconds_no_period() {
      // NB: I'm not sure if ffprobe returns data like this. Just covering all bases.
      let seconds = "123";
      let millis = parse_seconds(seconds).expect("should be able to parse seconds");
      assert_eq!(millis, 123000);
    }

    #[test]
    pub fn real_data() {
      // Duration: 00:00:26.23, start: 0.000000, bitrate: 1007 kb/s
      let seconds = "26.226200";
      let millis = parse_seconds(seconds).expect("should be able to parse seconds");
      assert_eq!(millis, 26226);
    }
  }
}
