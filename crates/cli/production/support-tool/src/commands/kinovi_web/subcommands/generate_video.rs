use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use clap::Args;
use log::info;

use artcraft_client::recipes::download_media_file::{
  download_media_file, DownloadMediaFileArgs, DownloadPath,
};
use artcraft_client::utils::api_host::ApiHost;
use kinovi_web_client::creds::kinovi_web_session::KinoviWebSession;
use kinovi_web_client::generate::video::generate_happy_horse_1p0::{
  generate_happy_horse_1p0, GenerateHappyHorse1p0Args, GenerateHappyHorse1p0Request,
};
use kinovi_web_client::generate::video::generate_seedance_2p0::{
  generate_seedance_2p0, GenerateSeedance2p0Args, GenerateSeedance2p0Request,
};
use kinovi_web_client::generate::video::generate_seedance_2p0_fast::{
  generate_seedance_2p0_fast, GenerateSeedance2p0FastArgs, GenerateSeedance2p0FastRequest,
};
use kinovi_web_client::requests::prepare_file_upload::prepare_file_upload::{
  prepare_file_upload, PrepareFileUploadArgs,
};
use kinovi_web_client::requests::upload_file::upload_file::{upload_file, UploadFileArgs};
use tokens::tokens::media_files::MediaFileToken;

use super::super::state::KinoviWebState;

/// Default cache location, relative to `$HOME`. NB: Not a shared `/tmp` path, where another local
/// user could pre-create the directory and plant files or symlinks.
const DEFAULT_DOWNLOAD_PATH_UNDER_HOME: &str = ".cache/storyteller-support-tool/media_files";

// ── Args ──

#[derive(Args)]
#[command(
  override_usage = "support-tool kinovi_web generate_video [OPTIONS] --prompt <PROMPT>",
  after_help = "\
EXAMPLES:
  support-tool kinovi_web generate_video --prompt \"A corgi at the lake\"
  support-tool kinovi_web generate_video --prompt prompt.txt --model happyhorse
  support-tool kinovi_web generate_video --prompt \"Dancing\" --start_frame_media_token mf_abc123
  support-tool kinovi_web generate_video --prompt \"Cat\" --image_reference_tokens \"mf_abc,mf_def\"
",
)]
pub struct GenerateVideoArgs {
  /// A prompt string or path to a file containing the prompt (.txt or .md).
  #[arg(long)]
  pub prompt: String,

  /// A single media token for the start frame image.
  #[arg(long)]
  pub start_frame_media_token: Option<String>,

  /// A single media token for the end frame image.
  #[arg(long)]
  pub end_frame_media_token: Option<String>,

  /// Comma or space separated media tokens for image references.
  #[arg(long)]
  pub image_reference_tokens: Option<String>,

  /// Comma or space separated media tokens for video references.
  #[arg(long)]
  pub video_reference_tokens: Option<String>,

  /// Comma or space separated media tokens for audio references.
  #[arg(long)]
  pub audio_reference_tokens: Option<String>,

  /// Use localhost:12345 instead of production API for artcraft_client lookups.
  #[arg(long)]
  pub localhost: bool,

  /// Directory to cache downloaded media files. Must be private to the current user (mode 0700).
  /// [default: ~/.cache/storyteller-support-tool/media_files]
  #[arg(long)]
  pub download_path: Option<String>,

  /// Model: seedance2p0, seedance2p0fast, happyhorse (and underscore variants).
  /// [default: seedance2p0]
  #[arg(long)]
  pub model: Option<String>,
}

// ── Model enum ──

#[derive(Debug, Clone, Copy)]
enum VideoModel {
  Seedance2p0,
  Seedance2p0Fast,
  HappyHorse,
}

fn parse_model(value: &str) -> anyhow::Result<VideoModel> {
  match value.to_lowercase().as_str() {
    "seedance2" | "seedance2p0" | "seedance_2p0" => Ok(VideoModel::Seedance2p0),
    "seedance2fast" | "seedance2p0fast" | "seedance_2p0_fast" => Ok(VideoModel::Seedance2p0Fast),
    "happyhorse" | "happy_horse" => Ok(VideoModel::HappyHorse),
    other => Err(anyhow!(
      "Unknown model '{}'. Valid: seedance2, seedance2p0, seedance_2p0, \
       seedance2fast, seedance2p0fast, seedance_2p0_fast, happyhorse, happy_horse",
      other
    )),
  }
}

// ── Resolved media inputs ──

struct ResolvedMediaInputs {
  start_frame_url: Option<String>,
  end_frame_url: Option<String>,
  reference_image_urls: Option<Vec<String>>,
  reference_video_urls: Option<Vec<String>>,
  reference_audio_urls: Option<Vec<String>>,
}

// ── Entry point ──

pub async fn run(state: &KinoviWebState, args: GenerateVideoArgs) -> anyhow::Result<()> {
  let session = KinoviWebSession::from_cookies_string(state.cookies.clone());
  let model = parse_model(args.model.as_deref().unwrap_or("seedance2p0"))?;
  let prompt = resolve_prompt(&args.prompt)?;
  let api_host = if args.localhost { ApiHost::Localhost { port: 12345 } } else { ApiHost::Storyteller };

  info!("Model: {:?}", model);
  info!("Prompt: {:?}", prompt);

  // Resolve download path.
  let download_dir = resolve_download_dir(args.download_path.as_deref())?;
  info!("Download cache directory: {:?}", download_dir);

  // Parse media tokens.
  let start_frame_token = args.start_frame_media_token.as_deref()
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .map(|s| MediaFileToken::new_from_str(s));

  let end_frame_token = args.end_frame_media_token.as_deref()
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .map(|s| MediaFileToken::new_from_str(s));

  let image_ref_tokens = parse_media_tokens(args.image_reference_tokens.as_deref());
  let video_ref_tokens = parse_media_tokens(args.video_reference_tokens.as_deref());
  let audio_ref_tokens = parse_media_tokens(args.audio_reference_tokens.as_deref());

  // Collect all tokens that need downloading.
  let mut all_tokens: Vec<MediaFileToken> = Vec::new();
  if let Some(t) = &start_frame_token { all_tokens.push(t.clone()); }
  if let Some(t) = &end_frame_token { all_tokens.push(t.clone()); }
  all_tokens.extend(image_ref_tokens.iter().cloned());
  all_tokens.extend(video_ref_tokens.iter().cloned());
  all_tokens.extend(audio_ref_tokens.iter().cloned());

  info!("Total media tokens to process: {}", all_tokens.len());

  // Download media files (with caching).
  let downloaded_files = download_media_files(&all_tokens, &api_host, &download_dir).await?;

  // Upload media files to Kinovi/KinoviWeb.
  let uploaded_urls = upload_media_files_to_kinovi(&session, &downloaded_files).await?;

  // Map uploaded URLs back to their roles.
  let mut url_iter = uploaded_urls.into_iter();

  let start_frame_url = if start_frame_token.is_some() { url_iter.next() } else { None };
  let end_frame_url = if end_frame_token.is_some() { url_iter.next() } else { None };

  let reference_image_urls: Vec<String> = url_iter.by_ref().take(image_ref_tokens.len()).collect();
  let reference_video_urls: Vec<String> = url_iter.by_ref().take(video_ref_tokens.len()).collect();
  let reference_audio_urls: Vec<String> = url_iter.by_ref().take(audio_ref_tokens.len()).collect();

  let media_inputs = ResolvedMediaInputs {
    start_frame_url,
    end_frame_url,
    reference_image_urls: if reference_image_urls.is_empty() { None } else { Some(reference_image_urls) },
    reference_video_urls: if reference_video_urls.is_empty() { None } else { Some(reference_video_urls) },
    reference_audio_urls: if reference_audio_urls.is_empty() { None } else { Some(reference_audio_urls) },
  };

  // Generate video.
  generate(model, &session, &prompt, media_inputs).await
}

// ── Prompt resolution ──

fn resolve_prompt(input: &str) -> anyhow::Result<String> {
  let looks_like_file = input.ends_with(".txt") || input.ends_with(".md");

  if looks_like_file {
    let path = Path::new(input);
    if !path.exists() {
      return Err(anyhow!("Prompt file does not exist: {:?}", path));
    }
    let content = fs::read_to_string(path)
      .map_err(|err| anyhow!("Failed to read prompt file {:?}: {}", path, err))?;
    let trimmed = content.trim().to_string();
    info!("Read prompt from file {:?} ({} chars)", path, trimmed.len());
    Ok(trimmed)
  } else {
    info!("Using prompt string directly ({} chars)", input.len());
    Ok(input.to_string())
  }
}

// ── Download path resolution ──

fn resolve_download_dir(maybe_path: Option<&str>) -> anyhow::Result<PathBuf> {
  let dir = match maybe_path {
    Some(path) => {
      let p = PathBuf::from(path);
      if p.exists() && !p.is_dir() {
        return Err(anyhow!("--download_path {:?} exists but is not a directory", p));
      }
      p
    }
    None => std::env::var_os("HOME")
      .map(|home| PathBuf::from(home).join(DEFAULT_DOWNLOAD_PATH_UNDER_HOME))
      .ok_or_else(|| anyhow!("HOME is not set; pass --download_path"))?,
  };

  if !dir.exists() {
    info!("Creating download directory: {:?}", dir);
    fs::DirBuilder::new()
      .recursive(true)
      .mode(0o700)
      .create(&dir)
      .map_err(|err| anyhow!("Failed to create download directory {:?}: {}", dir, err))?;
  }

  ensure_private_directory(&dir)?;

  Ok(dir)
}

/// Cached files are read and uploaded with the operator's authority, so the cache must not be
/// writable (or even readable) by anyone else. An owner-only directory belonging to someone else
/// is unreadable to us, so the mode check alone rules out foreign-controlled caches.
fn ensure_private_directory(dir: &Path) -> anyhow::Result<()> {
  let metadata = fs::symlink_metadata(dir)
    .map_err(|err| anyhow!("Cannot inspect download directory {:?}: {}", dir, err))?;
  if !metadata.is_dir() {
    return Err(anyhow!("Download path {:?} is not a directory (symlinks are not accepted)", dir));
  }
  if metadata.permissions().mode() & 0o077 != 0 {
    return Err(anyhow!("Download directory {:?} must be private to you (chmod 700)", dir));
  }
  Ok(())
}

// ── Media token parsing ──

fn parse_media_tokens(input: Option<&str>) -> Vec<MediaFileToken> {
  let input = match input {
    Some(s) if !s.trim().is_empty() => s,
    _ => return Vec::new(),
  };

  input
    .split(|c: char| c == ',' || c.is_whitespace())
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .map(|s| MediaFileToken::new_from_str(s))
    .collect()
}

// ── Download media files with caching ──

struct DownloadedFile {
  path: PathBuf,
  extension: String,
}

async fn download_media_files(
  tokens: &[MediaFileToken],
  api_host: &ApiHost,
  download_dir: &Path,
) -> anyhow::Result<Vec<DownloadedFile>> {
  let mut results = Vec::new();

  for token in tokens {
    // Check if file already exists in cache.
    let maybe_cached = find_cached_file(download_dir, token);

    if let Some(cached) = maybe_cached {
      info!(
        "Cache hit: {} already exists at {:?} ({} bytes)",
        token.as_str(),
        cached.path,
        fs::metadata(&cached.path).map(|m| m.len()).unwrap_or(0),
      );
      results.push(cached);
      continue;
    }

    info!("Downloading {} from API...", token.as_str());

    let result = download_media_file(DownloadMediaFileArgs {
      media_token: token,
      api_host,
      download_path: DownloadPath::Directory(download_dir),
    }).await.map_err(|err| anyhow!("Failed to download {}: {}", token.as_str(), err))?;

    let extension = result.downloaded_file_path
      .extension()
      .and_then(|e| e.to_str())
      .unwrap_or("bin")
      .to_string();

    info!(
      "Downloaded: {} -> {:?} ({} bytes, class: {})",
      token.as_str(),
      result.downloaded_file_path,
      result.filesize_bytes,
      result.media_file_response.media_file.media_class.to_str(),
    );

    results.push(DownloadedFile {
      path: result.downloaded_file_path,
      extension,
    });
  }

  Ok(results)
}

/// Look for an existing cached file matching the token (any extension).
fn find_cached_file(download_dir: &Path, token: &MediaFileToken) -> Option<DownloadedFile> {
  let prefix = format!("{}.", token.as_str());

  let entries = fs::read_dir(download_dir).ok()?;
  for entry in entries.flatten() {
    let filename = entry.file_name();
    let filename_str = filename.to_string_lossy();
    if filename_str.starts_with(&prefix) {
      let path = entry.path();
      // NB: `symlink_metadata` so a link to some other file is never treated as a cached download.
      let metadata = fs::symlink_metadata(&path).ok()?;
      if !metadata.is_file() {
        continue;
      }
      // Skip zero-byte files — treat as incomplete downloads.
      if metadata.len() == 0 {
        continue;
      }
      let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_string();
      return Some(DownloadedFile { path, extension });
    }
  }

  None
}

// ── Upload to Kinovi/KinoviWeb ──

async fn upload_media_files_to_kinovi(
  session: &KinoviWebSession,
  files: &[DownloadedFile],
) -> anyhow::Result<Vec<String>> {
  let mut urls = Vec::new();

  for file in files {
    info!("Uploading {:?} to Kinovi (extension: {})...", file.path, file.extension);

    // 1. Prepare upload (get signed URL).
    let prepare_result = prepare_file_upload(PrepareFileUploadArgs {
      session,
      extension: file.extension.clone(),
      host_override: None,
    }).await.map_err(|err| anyhow!("Failed to prepare upload for {:?}: {:?}", file.path, err))?;

    // 2. Read file bytes.
    let file_bytes = fs::read(&file.path)
      .map_err(|err| anyhow!("Failed to read {:?}: {}", file.path, err))?;

    info!("Uploading {} bytes to signed URL...", file_bytes.len());

    // 3. Upload to Cloudflare R2.
    let upload_result = upload_file(UploadFileArgs {
      upload_url: prepare_result.upload_url,
      file_bytes,
      host_override: None,
    }).await.map_err(|err| anyhow!("Failed to upload {:?}: {:?}", file.path, err))?;

    info!("Uploaded: {:?} -> {}", file.path, upload_result.public_url);
    urls.push(upload_result.public_url);
  }

  Ok(urls)
}

// ── Generate video ──

async fn generate(
  model: VideoModel,
  session: &KinoviWebSession,
  prompt: &str,
  media: ResolvedMediaInputs,
) -> anyhow::Result<()> {
  match model {
    VideoModel::HappyHorse => {
      // Happy Horse only supports start_frame_url.
      if media.end_frame_url.is_some() {
        return Err(anyhow!("Happy Horse does not support --end_frame_media_token"));
      }
      if media.reference_image_urls.is_some() {
        return Err(anyhow!("Happy Horse does not support --image_reference_tokens"));
      }
      if media.reference_video_urls.is_some() {
        return Err(anyhow!("Happy Horse does not support --video_reference_tokens"));
      }
      if media.reference_audio_urls.is_some() {
        return Err(anyhow!("Happy Horse does not support --audio_reference_tokens"));
      }

      info!("Generating video with Happy Horse 1.0...");
      let result = generate_happy_horse_1p0(GenerateHappyHorse1p0Args {
        request: GenerateHappyHorse1p0Request {
          prompt: prompt.to_string(),
          aspect_ratio: None,
          output_resolution: None,
          batch_count: None,
          duration_seconds: 5,
          start_frame_url: media.start_frame_url,
        },
        session,
        host_override: None,
      }).await.map_err(|err| anyhow!("Happy Horse generation failed: {:?}", err))?;

      info!("Success! task_id={}, order_id={}", result.task_id, result.order_id);
      if let Some(task_ids) = &result.task_ids {
        info!("All task_ids: {:?}", task_ids);
      }
      if let Some(order_ids) = &result.order_ids {
        info!("All order_ids: {:?}", order_ids);
      }
    }
    VideoModel::Seedance2p0 => {
      info!("Generating video with Seedance 2.0 Pro...");
      let result = generate_seedance_2p0(GenerateSeedance2p0Args {
        request: GenerateSeedance2p0Request {
          prompt: prompt.to_string(),
          aspect_ratio: None,
          output_resolution: None,
          duration_seconds: 5,
          batch_count: None,
          start_frame_url: media.start_frame_url,
          end_frame_url: media.end_frame_url,
          reference_image_urls: media.reference_image_urls,
          reference_video_urls: media.reference_video_urls,
          reference_audio_urls: media.reference_audio_urls,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
        session,
        host_override: None,
      }).await.map_err(|err| anyhow!("Seedance 2.0 generation failed: {:?}", err))?;

      info!("Success! task_id={}, order_id={}", result.task_id, result.order_id);
      if let Some(task_ids) = &result.task_ids {
        info!("All task_ids: {:?}", task_ids);
      }
      if let Some(order_ids) = &result.order_ids {
        info!("All order_ids: {:?}", order_ids);
      }
    }
    VideoModel::Seedance2p0Fast => {
      info!("Generating video with Seedance 2.0 Fast...");
      let result = generate_seedance_2p0_fast(GenerateSeedance2p0FastArgs {
        request: GenerateSeedance2p0FastRequest {
          prompt: prompt.to_string(),
          aspect_ratio: None,
          output_resolution: None,
          duration_seconds: 5,
          batch_count: None,
          start_frame_url: media.start_frame_url,
          end_frame_url: media.end_frame_url,
          reference_image_urls: media.reference_image_urls,
          reference_video_urls: media.reference_video_urls,
          reference_audio_urls: media.reference_audio_urls,
          character_ids: None,
          use_face_blur_hack: None,
          bitrate: None,
          maybe_generate_audio: None,
        },
        session,
        host_override: None,
      }).await.map_err(|err| anyhow!("Seedance 2.0 Fast generation failed: {:?}", err))?;

      info!("Success! task_id={}, order_id={}", result.task_id, result.order_id);
      if let Some(task_ids) = &result.task_ids {
        info!("All task_ids: {:?}", task_ids);
      }
      if let Some(order_ids) = &result.order_ids {
        info!("All order_ids: {:?}", order_ids);
      }
    }
  }

  Ok(())
}

#[cfg(test)]
mod download_dir_tests {
  use std::os::unix::fs::PermissionsExt;

  use super::ensure_private_directory;

  #[test]
  fn only_private_directories_are_accepted() {
    let dir = std::env::temp_dir().join(format!("support-tool-cache-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(ensure_private_directory(&dir).is_ok());

    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(ensure_private_directory(&dir).is_err());

    let link = dir.with_extension("link");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&dir, &link).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(ensure_private_directory(&link).is_err());

    std::fs::remove_file(&link).unwrap();
    std::fs::remove_dir(&dir).unwrap();
  }
}
