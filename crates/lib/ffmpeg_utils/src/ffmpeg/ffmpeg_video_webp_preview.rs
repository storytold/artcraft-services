use log::info;
use std::path::Path;
use std::process::Command;

use errors::AnyhowResult;

use crate::ffmpeg::run_thumbnail_ffmpeg::run_thumbnail_ffmpeg;
use crate::ffmpeg::webp_ffmpeg_binary::webp_ffmpeg;

/// Lossy WebP quality (0-100). Started at 60 on 2026-09-28; on the test video
/// this was ~50x smaller than the GIF preview (23 KB vs 1.1 MB). Safe to tune:
/// it only affects newly generated previews.
const WEBP_QUALITY: &str = "60";

pub struct FfmpegVideoWebpPreviewArgs<I: AsRef<Path>, O: AsRef<Path>> {
  pub input_video_path: I,
  pub output_webp_path: O,
}

/// Extract a short, endlessly looping animated WebP preview from a video file.
///
/// Uses the same frames as the GIF preview (`ffmpeg_video_gif_preview`): the
/// first 5 seconds, resampled to 10 fps, scaled to fit within 360x360
/// (preserving aspect ratio). Runs the ffmpeg chosen by
/// `webp_ffmpeg_binary::resolve_webp_ffmpeg` (call it at startup).
pub fn ffmpeg_video_webp_preview<I: AsRef<Path>, O: AsRef<Path>>(
  args: FfmpegVideoWebpPreviewArgs<I, O>,
) -> AnyhowResult<()> {
  let mut command = Command::new(webp_ffmpeg()?);

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
      .arg("fps=10,scale=360:360:force_original_aspect_ratio=decrease")
      .arg("-frames:v").arg("50")
      .arg("-an")
      .arg("-c:v").arg("libwebp_anim")
      .arg("-lossless").arg("0")
      .arg("-quality").arg(WEBP_QUALITY)
      // NB: The webp muxer plays animations once by default; 0 loops forever like the GIF.
      .arg("-loop").arg("0")
      .arg(args.output_webp_path.as_ref());

  info!("Calling ffmpeg (webp preview)...");

  run_thumbnail_ffmpeg(command)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ffmpeg::webp_ffmpeg_binary::resolve_webp_ffmpeg;
    use test_utils::test_file_path::test_file_path;

  #[test]
  fn test_extract_webp_preview_from_mp4() {
    if !libwebp_anim_available() {
      return;
    }

    let input_path = test_file_path("test_data/video/mp4/golden_sun_garoh.mp4")
        .expect("test video should exist");

    let temp_dir = tempfile::Builder::new().prefix("ffmpeg_webp_preview_test").tempdir_in("/tmp")
        .expect("should create temp dir");

    let output_path = temp_dir.path().join("preview.webp");

    ffmpeg_video_webp_preview(FfmpegVideoWebpPreviewArgs {
      input_video_path: &input_path,
      output_webp_path: &output_path,
    }).expect("ffmpeg should succeed");

    let bytes = std::fs::read(&output_path).expect("output webp should exist");

    // RIFF container holding a WebP file.
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WEBP");

    // Animated WebP: an ANIM chunk with a loop count of 0 (loop forever).
    let anim = find_chunk(&bytes, b"ANIM").expect("webp should be animated");
    let loop_count = u16::from_le_bytes([bytes[anim + 12], bytes[anim + 13]]);
    assert_eq!(loop_count, 0, "preview should loop forever");

    // More than one frame, and no more than the 50 we asked for.
    let frames = count_chunks(&bytes, b"ANMF");
    assert!(frames > 1 && frames <= 50, "unexpected frame count: {}", frames);

    // 640x480 input fits within 360x360 as 360x270. The VP8X chunk stores the
    // canvas width-1 and height-1 as 24-bit little-endian values. (Frames
    // after the first may be smaller sub-rectangles, so check the canvas.)
    let vp8x = find_chunk(&bytes, b"VP8X").expect("animated webp should have a VP8X header");
    let width = u24_le(&bytes[vp8x + 12..vp8x + 15]) + 1;
    let height = u24_le(&bytes[vp8x + 15..vp8x + 18]) + 1;
    assert_eq!((width, height), (360, 270));
  }

  /// Resolves like a development job would (including the macOS `ffmpeg-full`
  /// fallback). Skips if this machine has no webp-capable ffmpeg at all.
  fn libwebp_anim_available() -> bool {
    match resolve_webp_ffmpeg(true) {
      Ok(_) => true,
      Err(err) => {
        eprintln!("SKIPPED: {}", err);
        false
      }
    }
  }

  /// Offset of the first RIFF chunk with this FourCC, walking top-level chunks
  /// after the 12-byte RIFF/WEBP header.
  fn find_chunk(bytes: &[u8], fourcc: &[u8; 4]) -> Option<usize> {
    chunk_offsets(bytes).into_iter().find(|&offset| &bytes[offset..offset + 4] == fourcc)
  }

  fn count_chunks(bytes: &[u8], fourcc: &[u8; 4]) -> usize {
    chunk_offsets(bytes).into_iter().filter(|&offset| &bytes[offset..offset + 4] == fourcc).count()
  }

  fn chunk_offsets(bytes: &[u8]) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
      offsets.push(offset);
      let size = u32::from_le_bytes([bytes[offset + 4], bytes[offset + 5], bytes[offset + 6], bytes[offset + 7]]) as usize;
      // Chunk payloads are padded to an even length.
      offset += 8 + size + (size & 1);
    }
    offsets
  }

  fn u24_le(bytes: &[u8]) -> u32 {
    u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16)
  }
}
