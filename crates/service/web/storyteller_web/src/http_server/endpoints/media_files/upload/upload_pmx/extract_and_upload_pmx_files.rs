use std::collections::HashSet;
use std::io::{BufReader, Cursor, Read};
use std::path::PathBuf;

use log::{error, info, warn};
use once_cell::sync::Lazy;
use zip::ZipArchive;

use bucket_paths::legacy::typified_paths::public::media_files::bucket_file_path::MediaFileBucketPath;
use cloud_storage::legacy_bucket_client::LegacyBucketClient;
use filesys::path_to_string::path_to_string;
use hashing::sha256::sha256_hash_bytes::sha256_hash_bytes;
use mimetypes::mimetype_for_bytes::get_mimetype_for_bytes;


static ALLOWED_EXTENSIONS : Lazy<HashSet<&'static str>> = Lazy::new(|| {
  HashSet::from([
    ".pmx",
    ".png",
    ".tga",
  ])
});

/// Decompressed-size budgets (zip bombs). Generous for real PMX models plus textures.
const MAX_MEMBER_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug)]
pub enum PmxError {
  InvalidArchive,
  TooManyFiles,
  ExtractionError,
  NoPmxFile,
  UploadError,
  FileError,
}

pub struct PmxDetails {
  pub pmx_public_upload_path: MediaFileBucketPath,
  pub sha256_checksum: String,
  pub file_size_bytes: u64,
  pub maybe_mime_type: Option<String>,
}

pub async fn extract_and_upload_pmx_files(
  zip_container_file_bytes: &[u8],
  bucket_client: &LegacyBucketClient,
  prefix: Option<&str>,
  suffix: Option<&str>
) -> Result<PmxDetails, PmxError> {

  let maybe_mimetype = get_mimetype_for_bytes(zip_container_file_bytes);

  if maybe_mimetype != Some("application/zip") {
    warn!("File must be an application/zip");
    return Err(PmxError::InvalidArchive);
  }

  let mut cursor = Cursor::new(zip_container_file_bytes);
  let reader = std::io::BufReader::new(cursor);
  let mut archive = zip::ZipArchive::new(reader)
      .map_err(|err| {
        error!("Problem reading zip archive: {:?}", err);
        PmxError::InvalidArchive
      })?;

  if archive.len() > 255 {
    return Err(PmxError::TooManyFiles);
  }

  let pmx_public_upload_path = MediaFileBucketPath::generate_new(prefix, suffix);
  let pmx_public_upload_directory = pmx_public_upload_path.get_directory().get_directory_path_str();

  // TODO(bt): Fix these
  let mut hash = "";
  let mut file_size_bytes = 0;

  info!("Reading archive contents...");

  let entries = get_relevant_zip_entries(&mut archive)?;
  let mut total_bytes: u64 = 0;

  for entry in entries.iter() {
    info!("Entry: {:?}", entry);

    let enclosed_name = path_to_string(&entry.enclosed_name);
    let mut file = archive.by_name(&enclosed_name)
        .map_err(|err| {
          error!("Problem reading file from archive: {:?}", err);
          PmxError::InvalidArchive
        })?;

    let filename = file.name();
    let filename_lowercase = filename.to_lowercase();

    let mut zip_item_bytes = Vec::new();

    // Read at most one byte past the budget so an over-budget member is detected without expanding it.
    let limit = MAX_MEMBER_BYTES.min(MAX_TOTAL_BYTES - total_bytes);
    (&mut file).take(limit + 1).read_to_end(&mut zip_item_bytes)
        .map_err(|err| {
          error!("Problem reading file from archive: {:?}", err);
          PmxError::ExtractionError
        })?;

    if zip_item_bytes.len() as u64 > limit {
      warn!("Archive member exceeds decompressed size budget");
      return Err(PmxError::ExtractionError);
    }
    total_bytes += zip_item_bytes.len() as u64;

    if entry.is_pmx {
      bucket_client.upload_file_with_content_type(
        pmx_public_upload_path.get_full_object_path_str(),
        zip_item_bytes.as_ref(),
        "application/octet-stream")
          .await
          .map_err(|e| {
            warn!("Upload media bytes (pmx) to bucket error: {:?}", e);
            PmxError::UploadError
          })?;

      let hash = sha256_hash_bytes(&zip_item_bytes)
          .map_err(|io_error| {
            error!("Problem hashing bytes: {:?}", io_error);
            PmxError::FileError
          })?;

      file_size_bytes = zip_item_bytes.len();

    } else {
      let name = entry.maybe_better_alternative_output_name.as_ref()
          .unwrap_or_else(|| &entry.enclosed_name);
      let name = path_to_string(name);

      let path = format!("{}/{}", pmx_public_upload_directory, name);

      let mimetype = get_mimetype_for_bytes(&zip_item_bytes)
          .unwrap_or("application/octet-stream");

      bucket_client.upload_file_with_content_type(
        &path,
        zip_item_bytes.as_ref(),
        mimetype)
          .await
          .map_err(|e| {
            warn!("Upload media bytes (non-pmx) to bucket error: {:?}", e);
            PmxError::UploadError
          })?;
    }
  }

  Ok(PmxDetails {
    pmx_public_upload_path,
    sha256_checksum: hash.to_string(),
    file_size_bytes: file_size_bytes as u64,
    maybe_mime_type: Some("application/octet-stream".to_string()),
  })
}

#[derive(Debug, Clone)]
struct PmxZipEntryDetail {
  enclosed_name: PathBuf,
  maybe_better_alternative_output_name: Option<PathBuf>,
  file_size: u64,
  is_pmx: bool,
}

fn get_relevant_zip_entries(archive: &mut ZipArchive<BufReader<Cursor<&[u8]>>>) -> Result<Vec<PmxZipEntryDetail>, PmxError> {
  let mut entries = Vec::new();

  for i in 0..(archive.len()) {
    info!("Reading file {}...", i);

    let mut file = archive.by_index(i)
        .map_err(|err| {
          error!("Problem reading file from archive: {:?}", err);
          PmxError::InvalidArchive
        })?;

    let filename = file.name();
    let filename_lowercase = filename.to_lowercase();

    info!("File {} is {:?} - is file = {}", i, filename, file.is_file());
    info!("Enclosed name: {:?}", file.enclosed_name());

    if file.is_dir() {
      continue;
    }

    if filename_lowercase.starts_with("__macosx/") {
      // Mac users sometimes have a bogus __MACOSX directory, which may double the file count.
      continue;
    }

    let enclosed_name = match file.enclosed_name() {
      None => return Err(PmxError::FileError),
      Some(name) => name,
    };
    if enclosed_name
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
      return Err(PmxError::FileError);
    }

    if filename_lowercase.ends_with(".pmx") {
      entries.push(PmxZipEntryDetail {
        enclosed_name: enclosed_name.to_path_buf(),
        maybe_better_alternative_output_name: None,
        file_size: file.size(),
        is_pmx: true,
      });
    } else {
      // TODO(bt): Check type
      entries.push(PmxZipEntryDetail {
        enclosed_name: enclosed_name.to_path_buf(),
        maybe_better_alternative_output_name: None,
        file_size: file.size(),
        is_pmx: false,
      })
    }
  }

  let entries = keep_only_largest_pmx_file(entries)?;
  let entries = remove_useless_leading_directories(entries)?;

  for entry in entries.iter() {
    info!("Entry: {:?}", entry);
  }

  Ok(entries)
}

// Some PMX zip files contain multiple PMXes. As a heuristic, we want to keep the largest one.
fn keep_only_largest_pmx_file(mut entries: Vec<PmxZipEntryDetail>) -> Result<Vec<PmxZipEntryDetail>, PmxError> {
  let non_pmx_entries = entries.iter()
      .filter(|entry| !entry.is_pmx)
      .map(|entry| entry.clone())
      .collect::<Vec<PmxZipEntryDetail>>();

  let largest_pmx= entries.iter()
      .filter(|entry| entry.is_pmx)
      .reduce(|a, b| {
        if a.file_size > b.file_size {
          a
        } else {
          b
        }
      })
      .map(|entry| entry.clone());

  match largest_pmx {
    None => return Err(PmxError::NoPmxFile),
    Some(pmx_file) => {
      entries = non_pmx_entries;
      entries.push(pmx_file);
    }
  }

  Ok(entries)
}

// Some zip files have entries with useless leading directories. This will remove them.
fn remove_useless_leading_directories(mut entries: Vec<PmxZipEntryDetail>) -> Result<Vec<PmxZipEntryDetail>, PmxError> {
  let maybe_parent_directory_to_remove = {
    let pmx_entries = entries.iter()
        .filter(|entry| entry.is_pmx)
        .collect::<Vec<&PmxZipEntryDetail>>();

    match pmx_entries.get(0) {
      None => return Err(PmxError::NoPmxFile),
      Some(pmx_file) => pmx_file.enclosed_name.parent().map(|p| p.to_path_buf()),
    }
  };

  if let Some(parent) = maybe_parent_directory_to_remove {
    info!("Common parent: {:?}", parent);

    let remove_parent = entries.iter()
        .all(|entry| entry.enclosed_name.starts_with(&parent));

    if remove_parent {
      entries = entries.into_iter()
          .map(|entry| {
            // NB: `enclosed_name` may hold inner `..` (e.g. `a/../b`), which stripping `a` turns into
            // `../b`. Only keep a stripped name made of plain components, else the storage path
            // could leave the asset's directory.
            let maybe_new_path = entry.enclosed_name.strip_prefix(&parent)
                .ok()
                .filter(|path| path.components().all(|c| matches!(c, std::path::Component::Normal(_))))
                .map(|path| path.to_path_buf());
            PmxZipEntryDetail {
              enclosed_name: entry.enclosed_name,
              maybe_better_alternative_output_name: maybe_new_path,
              file_size: entry.file_size,
              is_pmx: entry.is_pmx,
            }
          })
          .collect::<Vec<PmxZipEntryDetail>>();
    }
  }

  let mut destinations = HashSet::with_capacity(entries.len());
  for entry in &entries {
    let destination = entry
        .maybe_better_alternative_output_name
        .as_ref()
        .unwrap_or(&entry.enclosed_name);
    if !destinations.insert(destination.clone()) {
      return Err(PmxError::FileError);
    }
  }

  Ok(entries)
}
