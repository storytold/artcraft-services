//! Mesh-only classification for Fal results. ZIP is a container, not a global
//! alias for OBJ. Fal can return an OBJ model wrapped in a ZIP package. The
//! package is validated and the single OBJ inside is extracted, so what gets
//! stored is a genuine raw OBJ that every existing mesh reader already handles.

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read};
use std::path::Path;

use enums::by_table::media_files::media_file_type::MediaFileType;
use mimetypes::mimetype_info::mimetype_info::MimetypeInfo;
use zip::ZipArchive;

use super::resolve_file_metadata::{resolve_file_metadata_with_file_name, MetadataSource};

const MAX_OBJ_PACKAGE_ENTRIES: usize = 1024;
// Limit on the sizes *declared* in the ZIP entry headers, i.e. what the package
// claims. Declared sizes are untrusted, so extraction enforces its own limit.
const MAX_OBJ_PACKAGE_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;
// Hard limit on the bytes actually read when extracting the OBJ. The webhook
// holds the downloaded ZIP, the extracted OBJ and the upload copy in memory at
// once, so keep this well below the old 256 MiB.
const MAX_EXTRACTED_OBJ_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct ResolvedMeshFileMetadata {
  pub mime_type: String,
  pub extension_with_period: &'static str,
  pub media_file_type: MediaFileType,
  pub source: MetadataSource,
  /// The caller must hash and store these extracted OBJ bytes, not the ZIP.
  pub replacement_file_bytes: Option<Vec<u8>>,
  /// Actual accepted MTL/texture entries that will not be stored; directories
  /// do not count. Kept for an explicit discard log in the uploader.
  pub discarded_companion_files: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum MeshMetadataError {
  UnrecognizedFileType,
  UnsupportedMimeType(String),
  InvalidZip,
  UnsafePath,
  UnsafeEntryMode,
  TooManyEntries,
  DeclaredPackageTooLarge,
  DisallowedFileType(String),
  MissingObj,
  EmptyObj,
  MultipleObjFiles,
  ExtractedObjTooLarge { max_bytes: u64 },
  DisguisedObj(String),
}

#[derive(Debug)]
struct ExtractedObjPackage {
  obj_bytes: Vec<u8>,
  discarded_companion_files: usize,
}

pub(super) fn resolve_mesh_file_metadata(file_bytes: &[u8], fal_content_type: Option<&str>, maybe_file_name: Option<&str>) -> Result<ResolvedMeshFileMetadata, MeshMetadataError> {
  // Inspect ZIP magic before the generic resolver: ZIP has no globally
  // supported FileExtension, so that resolver may fall back to misleading hints.
  let is_zip = MimetypeInfo::get_for_bytes(file_bytes).map(|info| info.mime_type() == "application/zip").unwrap_or(false);

  if is_zip {
    let package = extract_obj_from_package(file_bytes)?;
    // OBJ is plain text with no magic bytes. A recognizable nested format is
    // not an OBJ, whatever its name or the webhook's hints say.
    if let Some(info) = MimetypeInfo::get_for_bytes(&package.obj_bytes) {
      return Err(MeshMetadataError::DisguisedObj(info.mime_type().to_string()));
    }
    let mut resolved = resolve_non_zip(&package.obj_bytes, Some("model/obj"), Some("model.obj"))?;
    if resolved.media_file_type != MediaFileType::Obj {
      return Err(MeshMetadataError::DisguisedObj(resolved.mime_type));
    }
    resolved.replacement_file_bytes = Some(package.obj_bytes);
    resolved.discarded_companion_files = package.discarded_companion_files;
    return Ok(resolved);
  }

  resolve_non_zip(file_bytes, fal_content_type, maybe_file_name)
}

impl Display for MeshMetadataError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::UnrecognizedFileType => write!(f, "Could not determine file type for mesh"),
      Self::UnsupportedMimeType(mime_type) => write!(f, "Unsupported media file type: {}", mime_type),
      Self::InvalidZip => write!(f, "OBJ ZIP rejected: corrupt or unsupported ZIP data"),
      Self::UnsafePath => write!(f, "OBJ ZIP rejected: unsafe entry path"),
      Self::UnsafeEntryMode => write!(f, "OBJ ZIP rejected: symlink or special-file entry"),
      Self::TooManyEntries => write!(f, "OBJ ZIP rejected: more than {} entries", MAX_OBJ_PACKAGE_ENTRIES),
      Self::DeclaredPackageTooLarge => write!(f, "OBJ ZIP rejected: declared package size exceeds {} bytes", MAX_OBJ_PACKAGE_UNCOMPRESSED_BYTES),
      Self::DisallowedFileType(extension) => write!(f, "OBJ ZIP rejected: disallowed entry extension {:?}", extension),
      Self::MissingObj => write!(f, "OBJ ZIP rejected: no OBJ entry"),
      Self::EmptyObj => write!(f, "OBJ ZIP rejected: empty OBJ entry"),
      Self::MultipleObjFiles => write!(f, "OBJ ZIP rejected: more than one OBJ entry"),
      Self::ExtractedObjTooLarge { max_bytes } => write!(f, "OBJ ZIP rejected: extracted OBJ exceeds {} byte limit", max_bytes),
      Self::DisguisedObj(mime_type) => write!(f, "OBJ ZIP rejected: OBJ entry contains {} data", mime_type),
    }
  }
}

impl Error for MeshMetadataError {}

// Unrelated formats keep exactly the existing resolution and mapping.
fn resolve_non_zip(file_bytes: &[u8], fal_content_type: Option<&str>, maybe_file_name: Option<&str>) -> Result<ResolvedMeshFileMetadata, MeshMetadataError> {
  let metadata = resolve_file_metadata_with_file_name(file_bytes, fal_content_type, maybe_file_name).ok_or(MeshMetadataError::UnrecognizedFileType)?;
  let media_file_type = resolve_media_type(&metadata.mime_type)?;
  Ok(ResolvedMeshFileMetadata { mime_type: metadata.mime_type, extension_with_period: metadata.file_extension.extension_with_period(), media_file_type, source: metadata.source, replacement_file_bytes: None, discarded_companion_files: 0 })
}

fn resolve_media_type(mime_type: &str) -> Result<MediaFileType, MeshMetadataError> {
  MediaFileType::try_from_mime_type(mime_type).ok_or_else(|| MeshMetadataError::UnsupportedMimeType(mime_type.to_string()))
}

/// Validate a package and extract its OBJ. Companions are checked but not read
/// or retained; material/texture support needs separate maintainer signoff.
fn extract_obj_from_package(file_bytes: &[u8]) -> Result<ExtractedObjPackage, MeshMetadataError> {
  extract_obj_from_package_with_limit(file_bytes, MAX_EXTRACTED_OBJ_BYTES)
}

fn extract_obj_from_package_with_limit(file_bytes: &[u8], max_extracted_bytes: u64) -> Result<ExtractedObjPackage, MeshMetadataError> {
  let mut archive = ZipArchive::new(Cursor::new(file_bytes)).map_err(|_| MeshMetadataError::InvalidZip)?;
  if archive.len() > MAX_OBJ_PACKAGE_ENTRIES {
    return Err(MeshMetadataError::TooManyEntries);
  }

  let mut obj_index = None;
  let mut total_size = 0u64;
  let mut discarded_companion_files = 0;
  for index in 0..archive.len() {
    let file = archive.by_index(index).map_err(|_| MeshMetadataError::InvalidZip)?;
    if file.enclosed_name().is_none() || file.name().contains('\\') {
      return Err(MeshMetadataError::UnsafePath);
    }
    // Only directories and regular files, never symlinks or special files.
    if let Some(mode) = file.unix_mode() {
      let kind = mode & 0o170000;
      if kind != 0 && kind != 0o100000 && kind != 0o040000 {
        return Err(MeshMetadataError::UnsafeEntryMode);
      }
    }
    if file.is_dir() {
      continue;
    }
    // `size()` is the uncompressed size declared in the header, not measured.
    total_size = total_size.checked_add(file.size()).ok_or(MeshMetadataError::DeclaredPackageTooLarge)?;
    if total_size > MAX_OBJ_PACKAGE_UNCOMPRESSED_BYTES {
      return Err(MeshMetadataError::DeclaredPackageTooLarge);
    }
    let extension = Path::new(file.name()).extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
      "obj" => {
        if obj_index.is_some() {
          return Err(MeshMetadataError::MultipleObjFiles);
        }
        if file.size() == 0 {
          return Err(MeshMetadataError::EmptyObj);
        }
        obj_index = Some(index);
      },
      "mtl" | "png" | "jpg" | "jpeg" | "webp" => discarded_companion_files += 1,
      _ => return Err(MeshMetadataError::DisallowedFileType(extension)),
    }
  }

  // The declared size is untrusted, so bound the bytes actually read.
  let mut obj_file = archive.by_index(obj_index.ok_or(MeshMetadataError::MissingObj)?).map_err(|_| MeshMetadataError::InvalidZip)?;
  let mut obj_bytes = Vec::new();
  Read::take(&mut obj_file, max_extracted_bytes.saturating_add(1)).read_to_end(&mut obj_bytes).map_err(|_| MeshMetadataError::InvalidZip)?;
  if obj_bytes.is_empty() {
    return Err(MeshMetadataError::EmptyObj);
  }
  if obj_bytes.len() as u64 > max_extracted_bytes {
    return Err(MeshMetadataError::ExtractedObjTooLarge { max_bytes: max_extracted_bytes });
  }
  Ok(ExtractedObjPackage { obj_bytes, discarded_companion_files })
}

#[cfg(test)]
mod tests {
  use std::io::Write;

  use zip::write::FileOptions;
  use zip::ZipWriter;

  use super::*;

  const OBJ: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

  mod obj_packages {
    use super::*;

    #[test]
    fn obj_package_is_extracted_to_raw_obj() {
      let bytes = package(&[("model/model.OBJ", OBJ), ("model/material.mtl", b"newmtl material"), ("model/texture.png", b"texture")]);
      let metadata = resolve_mesh_file_metadata(&bytes, Some("application/zip"), Some("model.zip")).unwrap();
      assert_eq!(metadata.media_file_type, MediaFileType::Obj);
      assert_eq!(metadata.mime_type, "model/obj");
      assert_eq!(metadata.extension_with_period, ".obj");
      assert_eq!(metadata.replacement_file_bytes.as_deref(), Some(OBJ));
    }

    #[test]
    fn obj_package_extracted_without_or_with_misleading_hints() {
      let bytes = package(&[("model.obj", OBJ)]);
      for (content_type, file_name) in [(None, None), (Some("model/obj"), Some("model.obj")), (Some("application/octet-stream"), None)] {
        let metadata = resolve_mesh_file_metadata(&bytes, content_type, file_name).unwrap();
        assert_eq!(metadata.media_file_type, MediaFileType::Obj);
        assert_eq!(metadata.replacement_file_bytes.as_deref(), Some(OBJ));
      }
    }

    #[test]
    fn extraction_is_bounded_by_bytes_actually_read() {
      let bytes = package(&[("model.obj", OBJ)]);
      assert!(extract_obj_from_package_with_limit(&bytes, OBJ.len() as u64).is_ok());
      assert_eq!(extract_obj_from_package_with_limit(&bytes, OBJ.len() as u64 - 1).unwrap_err(), MeshMetadataError::ExtractedObjTooLarge { max_bytes: OBJ.len() as u64 - 1 });
    }

    #[test]
    fn valid_obj_variants_are_extracted_unchanged() {
      let comment: &[u8] = b"# exported by a tool\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
      let bom: &[u8] = b"\xEF\xBB\xBFv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
      let crlf: &[u8] = b"v 0 0 0\r\nv 1 0 0\r\nv 0 1 0\r\nf 1 2 3\r\n";
      let comment_crlf: &[u8] = b"# exported by a tool\r\nmtllib model.mtl\r\nv 0 0 0\r\nf 1 1 1\r\n";
      let bom_comment_crlf: &[u8] = b"\xEF\xBB\xBF# exported by a tool\r\nv 0 0 0\r\nf 1 1 1\r\n";
      for obj in [comment, bom, crlf, comment_crlf, bom_comment_crlf] {
        let bytes = package(&[("model.obj", obj)]);
        let metadata = resolve_mesh_file_metadata(&bytes, Some("application/zip"), Some("model.zip")).expect("valid OBJ variant rejected");
        assert_eq!(metadata.media_file_type, MediaFileType::Obj);
        assert_eq!(metadata.replacement_file_bytes.as_deref(), Some(obj));
      }
    }

    #[test]
    fn nested_archive_inside_obj_entry_rejected() {
      let inner = package(&[("model.obj", OBJ)]);
      let bytes = package(&[("model.obj", &inner)]);
      assert_eq!(resolve_mesh_file_metadata(&bytes, Some("application/zip"), None).unwrap_err(), MeshMetadataError::DisguisedObj("application/zip".to_string()));
    }

    #[test]
    fn discarded_companion_count_excludes_obj_and_directories() {
      let bytes = package(&[("model.obj", OBJ), ("textures/", b""), ("material.mtl", b"newmtl material"), ("textures/texture.png", b"texture")]);
      assert_eq!(resolve_mesh_file_metadata(&bytes, None, None).unwrap().discarded_companion_files, 2);
      let bytes = package(&[("model.obj", OBJ)]);
      assert_eq!(resolve_mesh_file_metadata(&bytes, None, None).unwrap().discarded_companion_files, 0);
    }
  }

  mod existing_formats {
    use super::*;

    #[test]
    fn supported_obj_responses_are_unchanged() {
      for (content_type, file_name) in [(Some("model/obj"), None), (None, Some("model.OBJ")), (Some("application/octet-stream"), Some("model.obj"))] {
        let metadata = resolve_mesh_file_metadata(OBJ, content_type, file_name).unwrap();
        assert_eq!(metadata.media_file_type, MediaFileType::Obj);
        assert_eq!(metadata.mime_type, "model/obj");
        assert_eq!(metadata.extension_with_period, ".obj");
      }
    }

    #[test]
    fn existing_fbx_and_glb_responses_are_unchanged() {
      for (content_type, file_name, expected) in [("model/fbx", "model.fbx", MediaFileType::Fbx), ("model/gltf-binary", "model.glb", MediaFileType::Glb), ("model/ply", "model.ply", MediaFileType::Ply)] {
        assert_eq!(resolve_mesh_file_metadata(b"unknown bytes", Some(content_type), Some(file_name)).unwrap().media_file_type, expected);
      }
    }

    #[test]
    fn non_archive_magic_still_takes_priority() {
      let png = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];
      let metadata = resolve_mesh_file_metadata(&png, Some("application/zip"), Some("model.obj")).unwrap();
      assert_eq!(metadata.media_file_type, MediaFileType::Png);
      assert_eq!(metadata.source, MetadataSource::MagicBytes);
    }
  }

  mod unsupported_files {
    use super::*;

    #[test]
    fn zip_is_not_a_global_obj_alias() {
      assert_eq!(MediaFileType::try_from_mime_type("application/zip"), None);
    }

    #[test]
    fn unrelated_archives_rejected_even_with_obj_hints() {
      for entries in [vec![("document.txt", b"document".as_slice())], vec![("model.glb", b"glTF".as_slice())], vec![("material.mtl", b"newmtl material".as_slice())]] {
        let bytes = package(&entries);
        assert!(resolve_mesh_file_metadata(&bytes, Some("application/zip"), Some("model.zip")).is_err());
        assert!(resolve_mesh_file_metadata(&bytes, Some("model/obj"), Some("model.obj")).is_err());
      }
    }

    #[test]
    fn unsafe_or_unsupported_package_contents_rejected() {
      for (extra, expected) in [("../texture.png", MeshMetadataError::UnsafePath), ("/texture.png", MeshMetadataError::UnsafePath), ("textures\\texture.png", MeshMetadataError::UnsafePath), ("program.exe", MeshMetadataError::DisallowedFileType("exe".to_string())), ("second.obj", MeshMetadataError::MultipleObjFiles)] {
        let bytes = package(&[("model.obj", OBJ), (extra, OBJ)]);
        assert_eq!(resolve_mesh_file_metadata(&bytes, Some("application/zip"), None).unwrap_err(), expected, "wrong reason for {}", extra);
      }
    }

    #[test]
    fn empty_obj_and_empty_archive_rejected() {
      assert_eq!(resolve_mesh_file_metadata(&package(&[("model.obj", b"")]), Some("application/zip"), None).unwrap_err(), MeshMetadataError::EmptyObj);
      assert_eq!(resolve_mesh_file_metadata(&package(&[]), Some("application/zip"), None).unwrap_err(), MeshMetadataError::MissingObj);
    }

    #[test]
    fn corrupt_zip_rejected_even_with_obj_hints() {
      let mut bytes = package(&[("model.obj", OBJ)]);
      bytes.truncate(40);
      assert_eq!(resolve_mesh_file_metadata(&bytes, Some("model/obj"), Some("model.obj")).unwrap_err(), MeshMetadataError::InvalidZip);
    }

    #[test]
    fn generic_resolver_does_not_accept_obj_packages() {
      let bytes = package(&[("model.obj", OBJ)]);
      assert!(resolve_file_metadata_with_file_name(&bytes, Some("application/zip"), Some("model.zip")).is_none());
    }

    #[test]
    fn too_many_package_entries_rejected() {
      let mut entries = vec![("material.mtl", b"newmtl material".as_slice()); MAX_OBJ_PACKAGE_ENTRIES];
      entries.push(("model.obj", OBJ));
      assert_eq!(resolve_mesh_file_metadata(&package(&entries), Some("application/zip"), None).unwrap_err(), MeshMetadataError::TooManyEntries);
    }

    #[test]
    fn symlink_in_package_rejected() {
      let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
      writer.start_file("model.obj", FileOptions::default()).unwrap();
      writer.write_all(OBJ).unwrap();
      writer.add_symlink("texture.png", "outside.png", FileOptions::default()).unwrap();
      let bytes = writer.finish().unwrap().into_inner();
      assert_eq!(resolve_mesh_file_metadata(&bytes, Some("application/zip"), None).unwrap_err(), MeshMetadataError::UnsafeEntryMode);
    }

    #[test]
    fn unknown_media_remains_unsupported() {
      assert_eq!(resolve_mesh_file_metadata(b"unknown bytes", Some("application/x-unsupported"), Some("file.xyz")).unwrap_err(), MeshMetadataError::UnrecognizedFileType);
    }

    #[test]
    fn declared_package_size_limit_is_reported() {
      let mut bytes = package(&[("model.obj", OBJ)]);
      // Change the declared size only; do not allocate a huge test fixture.
      let declared = (MAX_OBJ_PACKAGE_UNCOMPRESSED_BYTES as u32 + 1).to_le_bytes();
      let central = bytes.windows(4).position(|window| window == b"PK\x01\x02").unwrap();
      bytes[central + 24..central + 28].copy_from_slice(&declared);
      bytes[22..26].copy_from_slice(&declared);
      assert_eq!(resolve_mesh_file_metadata(&bytes, None, None).unwrap_err(), MeshMetadataError::DeclaredPackageTooLarge);
    }

    #[test]
    fn disguised_image_obj_is_rejected_with_mime() {
      let png: &[u8] = &[137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];
      let bytes = package(&[("model.obj", png)]);
      assert_eq!(resolve_mesh_file_metadata(&bytes, None, None).unwrap_err(), MeshMetadataError::DisguisedObj("image/png".to_string()));
    }

    #[test]
    fn unsupported_mime_preserves_its_diagnostic() {
      let error = resolve_media_type("application/zip").unwrap_err();
      assert_eq!(error, MeshMetadataError::UnsupportedMimeType("application/zip".to_string()));
      assert_eq!(error.to_string(), "Unsupported media file type: application/zip");
    }
  }

  fn package(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in entries {
      writer.start_file(*name, FileOptions::default()).unwrap();
      writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
  }
}
