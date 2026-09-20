use log::info;
use std::path::Path;
use std::process::Command;

use errors::AnyhowResult;

use crate::ffmpeg::run_thumbnail_ffmpeg::run_thumbnail_ffmpeg;

pub struct FfmpegVideoGifPreviewArgs<I: AsRef<Path>, O: AsRef<Path>> {
  pub input_video_path: I,
  pub output_gif_path: O,
}

/// Extract a short animated GIF preview from a video file.
///
/// Takes the first 5 seconds, resampled to 10 fps, scaled to fit within 360x360
/// (preserving aspect ratio), with an optimized color palette.
pub fn ffmpeg_video_gif_preview<I: AsRef<Path>, O: AsRef<Path>>(
  args: FfmpegVideoGifPreviewArgs<I, O>,
) -> AnyhowResult<()> {
  let mut command = Command::new("ffmpeg");

  command
      .arg("-nostdin")
      .arg("-y")
      .arg("-loglevel").arg("error")
      .arg("-nostats")
      .arg("-threads").arg("1")
      .arg("-filter_complex_threads").arg("1")
      .arg("-ss").arg("0")
      .arg("-to").arg("5")
      .arg("-i").arg(args.input_video_path.as_ref())
      .arg("-threads").arg("1")
      .arg("-filter_complex")
      .arg("fps=10,scale=360:360:force_original_aspect_ratio=decrease[s]; [s]split[a][b]; [a]palettegen[palette]; [b][palette]paletteuse")
      .arg("-frames:v").arg("50")
      .arg(args.output_gif_path.as_ref());

  info!("Calling ffmpeg (gif preview)...");

  run_thumbnail_ffmpeg(command)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempdir::TempDir;
  use test_utils::test_file_path::test_file_path;

  #[test]
  fn test_extract_gif_preview_from_mp4() {
    let input_path = test_file_path("test_data/video/mp4/golden_sun_garoh.mp4")
        .expect("test video should exist");

    let temp_dir = TempDir::new_in("/tmp", "ffmpeg_gif_preview_test")
        .expect("should create temp dir");

    let output_path = temp_dir.path().join("preview.gif");

    ffmpeg_video_gif_preview(FfmpegVideoGifPreviewArgs {
      input_video_path: &input_path,
      output_gif_path: &output_path,
    }).expect("ffmpeg should succeed");

    assert!(output_path.exists(), "output gif should exist");

    // The input video is 640x480. Fitting within 360x360 should produce 360x270
    // (maintaining 4:3 aspect ratio).
    // Use ffprobe to verify the GIF dimensions.
    let result = ffprobe::ffprobe(&output_path)
        .expect("ffprobe should succeed");

    let video_stream = result.streams.iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .expect("should have a video stream");

    assert_eq!(video_stream.width, Some(360));
    assert_eq!(video_stream.height, Some(270));
  }

  #[test]
  fn test_tall_video_preview_has_bounded_dimensions_and_frame_count() {
    let temp_dir = TempDir::new_in("/tmp", "ffmpeg_tall_gif_test").unwrap();
    let input = temp_dir.path().join("tall.mp4");
    let output = temp_dir.path().join("preview.gif");
    let status = Command::new("ffmpeg")
      .args(["-nostdin", "-y", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=32x640:rate=10", "-t", "6", "-threads", "1"])
      .arg(&input)
      .status().unwrap();
    assert!(status.success());

    ffmpeg_video_gif_preview(FfmpegVideoGifPreviewArgs {
      input_video_path: &input,
      output_gif_path: &output,
    }).unwrap();
    let probe = Command::new("ffprobe")
      .args(["-v", "error", "-count_frames", "-select_streams", "v:0", "-show_entries", "stream=width,height,nb_read_frames", "-of", "csv=p=0"])
      .arg(&output)
      .output().unwrap();
    assert!(probe.status.success());
    let values: Vec<u32> = std::str::from_utf8(&probe.stdout).unwrap().trim()
      .split(',').map(|value| value.parse().unwrap()).collect();
    assert_eq!(&values[..2], &[18, 360]);
    assert!(values[2] > 0 && values[2] <= 50);
  }
}
