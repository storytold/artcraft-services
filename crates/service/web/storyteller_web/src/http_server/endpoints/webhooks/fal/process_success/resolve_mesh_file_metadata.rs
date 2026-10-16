//! Mesh-only classification for Fal results. ZIP is a container, not a global
//! alias for OBJ: only an archive containing an OBJ and supported companion
//! files is accepted here. Preserve its ZIP MIME type and extension on storage.

use std::io::Cursor;
use std::path::Path;

use enums::by_table::media_files::media_file_type::MediaFileType;
use mimetypes::mimetype_info::mimetype_info::MimetypeInfo;
use zip::ZipArchive;

use super::resolve_file_metadata::{resolve_file_metadata_with_file_name, MetadataSource};

const MAX_OBJ_PACKAGE_ENTRIES: usize = 1024;
const MAX_OBJ_PACKAGE_UNCOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;

pub(super) struct ResolvedMeshFileMetadata {
  pub mime_type: String,
  pub extension_with_period: &'static str,
  pub media_file_type: MediaFileType,
  pub source: MetadataSource,
}

pub(super) fn resolve_mesh_file_metadata(file_bytes: &[u8], fal_content_type: Option<&str>, maybe_file_name: Option<&str>) -> Option<ResolvedMeshFileMetadata> {
  // Inspect ZIP magic before the generic resolver: ZIP has no globally
  // supported FileExtension, so that resolver may fall back to a misleading
  // webhook MIME type or filename (e.g. model/obj or model.obj).
  let is_zip = MimetypeInfo::get_for_bytes(file_bytes).map(|info| info.mime_type() == "application/zip").unwrap_or(false);

  if is_zip {
    if !is_supported_obj_package(file_bytes) {
      return None;
    }
    return Some(ResolvedMeshFileMetadata { mime_type: "application/zip".to_string(), extension_with_period: ".zip", media_file_type: MediaFileType::Obj, source: MetadataSource::MagicBytes });
  }

  // Unrelated formats keep exactly the existing resolution and mapping.
  let metadata = resolve_file_metadata_with_file_name(file_bytes, fal_content_type, maybe_file_name)?;
  let media_file_type = MediaFileType::try_from_mime_type(&metadata.mime_type)?;
  Some(ResolvedMeshFileMetadata { mime_type: metadata.mime_type, extension_with_period: metadata.file_extension.extension_with_period(), media_file_type, source: metadata.source })
}

/// Check package structure without extracting or decompressing untrusted files.
/// Support a single nonempty OBJ plus optional MTLs and texture images, not
/// arbitrary archives, multi-model bundles, or other mesh/project formats.
fn is_supported_obj_package(file_bytes: &[u8]) -> bool {
  let Ok(mut archive) = ZipArchive::new(Cursor::new(file_bytes)) else {
    return false;
  };
  if archive.len() > MAX_OBJ_PACKAGE_ENTRIES {
    return false;
  }

  let mut obj_count = 0;
  let mut total_size = 0u64;
  for index in 0..archive.len() {
    let Ok(file) = archive.by_index(index) else {
      return false;
    };
    if file.enclosed_name().is_none() || file.name().contains('\\') {
      return false;
    }
    // Only directories and regular files, never symlinks or special files.
    if let Some(mode) = file.unix_mode() {
      let kind = mode & 0o170000;
      if kind != 0 && kind != 0o100000 && kind != 0o040000 {
        return false;
      }
    }
    if file.is_dir() {
      continue;
    }
    let Some(size) = total_size.checked_add(file.size()) else {
      return false;
    };
    total_size = size;
    if total_size > MAX_OBJ_PACKAGE_UNCOMPRESSED_BYTES {
      return false;
    }
    let extension = Path::new(file.name()).extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
      "obj" if file.size() > 0 => obj_count += 1,
      "mtl" | "png" | "jpg" | "jpeg" | "webp" => {},
      _ => return false,
    }
  }
  obj_count == 1
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
    fn application_zip_obj_package_keeps_container_metadata() {
      let bytes = package(&[("model/model.OBJ", OBJ), ("model/material.mtl", b"newmtl material"), ("model/texture.png", b"texture")]);
      let metadata = resolve_mesh_file_metadata(&bytes, Some("application/zip"), Some("model.zip")).unwrap();
      assert_eq!(metadata.media_file_type, MediaFileType::Obj);
      assert_eq!(metadata.mime_type, "application/zip");
      assert_eq!(metadata.extension_with_period, ".zip");
      assert_eq!(metadata.source, MetadataSource::MagicBytes);
    }

    #[test]
    fn obj_package_detected_without_webhook_hints() {
      let bytes = package(&[("model.obj", OBJ)]);
      assert_eq!(resolve_mesh_file_metadata(&bytes, None, None).unwrap().media_file_type, MediaFileType::Obj);
    }

    #[test]
    fn archive_magic_wins_over_obj_hints() {
      let bytes = package(&[("model.obj", OBJ)]);
      let metadata = resolve_mesh_file_metadata(&bytes, Some("model/obj"), Some("model.obj")).unwrap();
      assert_eq!(metadata.mime_type, "application/zip");
      assert_eq!(metadata.extension_with_period, ".zip");
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
      for (content_type, file_name, expected) in [("model/fbx", "model.fbx", MediaFileType::Fbx), ("model/gltf-binary", "model.glb", MediaFileType::Glb)] {
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
        assert!(resolve_mesh_file_metadata(&bytes, Some("application/zip"), Some("model.zip")).is_none());
        assert!(resolve_mesh_file_metadata(&bytes, Some("model/obj"), Some("model.obj")).is_none());
      }
    }

    #[test]
    fn unsafe_or_unsupported_package_contents_rejected() {
      for extra in ["../texture.png", "/texture.png", "textures\\texture.png", "program.exe", "second.obj"] {
        let bytes = package(&[("model.obj", OBJ), (extra, OBJ)]);
        assert!(resolve_mesh_file_metadata(&bytes, Some("application/zip"), None).is_none(), "accepted {}", extra);
      }
    }

    #[test]
    fn empty_obj_and_empty_archive_rejected() {
      assert!(resolve_mesh_file_metadata(&package(&[("model.obj", b"")]), Some("application/zip"), None).is_none());
      assert!(resolve_mesh_file_metadata(&package(&[]), Some("application/zip"), None).is_none());
    }

    #[test]
    fn corrupt_zip_rejected_even_with_obj_hints() {
      let mut bytes = package(&[("model.obj", OBJ)]);
      bytes.truncate(40);
      assert!(resolve_mesh_file_metadata(&bytes, Some("model/obj"), Some("model.obj")).is_none());
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
      assert!(resolve_mesh_file_metadata(&package(&entries), Some("application/zip"), None).is_none());
    }

    #[test]
    fn symlink_in_package_rejected() {
      let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
      writer.start_file("model.obj", FileOptions::default()).unwrap();
      writer.write_all(OBJ).unwrap();
      writer.add_symlink("texture.png", "outside.png", FileOptions::default()).unwrap();
      let bytes = writer.finish().unwrap().into_inner();
      assert!(resolve_mesh_file_metadata(&bytes, Some("application/zip"), None).is_none());
    }

    #[test]
    fn unknown_media_remains_unsupported() {
      assert!(resolve_mesh_file_metadata(b"unknown bytes", Some("application/x-unsupported"), Some("file.xyz")).is_none());
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
