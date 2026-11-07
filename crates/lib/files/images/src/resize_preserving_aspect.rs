use image::DynamicImage;
use image::imageops::FilterType;

/// Resize an image preserving the exact aspect ratio of the source image.
///
/// If `exact_fit` is true, the image will be resized to fit the largest dimension,
/// then cropped to fit the precise dimensions specified.
pub fn resize_preserving_aspect(
  source_image: &DynamicImage,
  new_width: u32,
  new_height: u32,
  exact_fit: bool,
) -> DynamicImage {
  if exact_fit && new_width > 0 && new_height > 0 {
    // NB: Crop the source to the target aspect first. `resize_to_fill` scales the whole source up
    // to cover the target, so an extreme aspect ratio needed gigabytes for the intermediate.
    let (src_w, src_h) = (source_image.width() as u64, source_image.height() as u64);
    let (dst_w, dst_h) = (new_width as u64, new_height as u64);
    let (crop_w, crop_h) = if src_w * dst_h > dst_w * src_h {
      ((src_h * dst_w / dst_h).clamp(1, src_w), src_h)
    } else {
      (src_w, (src_w * dst_h / dst_w).clamp(1, src_h))
    };
    source_image
        .crop_imm(((src_w - crop_w) / 2) as u32, ((src_h - crop_h) / 2) as u32, crop_w as u32, crop_h as u32)
        .resize_exact(new_width, new_height, FilterType::Lanczos3)
  } else if exact_fit {
    source_image.resize_to_fill(new_width, new_height, FilterType::Lanczos3)
  } else {
    source_image.resize(new_width, new_height, FilterType::Lanczos3)
  }
}

#[cfg(test)]
mod tests {
  use image::DynamicImage;

  use errors::AnyhowResult;
  use test_utils::test_file_path::test_file_path;

  use crate::resize_preserving_aspect::resize_preserving_aspect;

  fn open_test_image(filename: &str) -> AnyhowResult<DynamicImage> {
    let path = test_file_path(filename)?;
    Ok(image::open(path)?)
  }

  fn open_tall_image() -> AnyhowResult<DynamicImage> {
    let image = open_test_image("test_data/image/mochi.jpg").unwrap();
    assert!(image.width() < image.height());
    assert_eq!(image.width(), 753);
    assert_eq!(image.height(), 1000);
    Ok(image)
  }

  fn open_wide_image() -> AnyhowResult<DynamicImage> {
    let image = open_test_image("test_data/image/juno.jpg").unwrap();
    assert!(image.width() > image.height());
    assert_eq!(image.width(), 1000);
    assert_eq!(image.height(), 750);
    Ok(image)
  }

  #[test]
  fn downsize_wide_image_preserving_aspect() {
    let image = open_wide_image().unwrap();
    let resized = resize_preserving_aspect(&image, 500, 500, false);
    assert!(resized.width() > resized.height());
    assert_eq!(resized.width(), 500);
    assert_eq!(resized.height(), 375);
  }

  #[test]
  fn downsize_tall_image_preserving_aspect() {
    let image = open_tall_image().unwrap();
    let resized = resize_preserving_aspect(&image, 500, 500, false);
    assert!(resized.width() < resized.height());
    assert_eq!(resized.width(), 377);
    assert_eq!(resized.height(), 500);
  }

  #[test]
  fn enlarge_wide_image_preserving_aspect() {
    let image = open_wide_image().unwrap();
    let resized = resize_preserving_aspect(&image, 1200, 1200, false);
    assert!(resized.width() > resized.height());
    assert_eq!(resized.width(), 1200);
    assert_eq!(resized.height(), 900);
  }

  #[test]
  fn enlarge_tall_image_preserving_aspect() {
    let image = open_tall_image().unwrap();
    let resized = resize_preserving_aspect(&image, 1200, 1200, false);
    assert!(resized.width() < resized.height());
    assert_eq!(resized.width(), 904);
    assert_eq!(resized.height(), 1200);
  }

  #[test]
  fn resize_wide_image_preserving_aspect_exact() {
    // Downscale
    let image = open_wide_image().unwrap();
    let resized = resize_preserving_aspect(&image, 500, 500, true);
    assert_eq!(resized.width(), 500);
    assert_eq!(resized.height(), 500);

    // Upscale
    let resized = resize_preserving_aspect(&image, 1200, 1200, true);
    assert_eq!(resized.width(), 1200);
    assert_eq!(resized.height(), 1200);

    // Really wide
    let resized = resize_preserving_aspect(&image, 1200, 100, true);
    assert_eq!(resized.width(), 1200);
    assert_eq!(resized.height(), 100);

    // Really tall
    let resized = resize_preserving_aspect(&image, 100, 1200, true);
    assert_eq!(resized.width(), 100);
    assert_eq!(resized.height(), 1200);
  }

  #[test]
  fn resize_tall_image_preserving_aspect_exact() {
    // Downscale
    let image = open_tall_image().unwrap();
    let resized = resize_preserving_aspect(&image, 500, 500, true);
    assert_eq!(resized.width(), 500);
    assert_eq!(resized.height(), 500);

    // Upscale
    let resized = resize_preserving_aspect(&image, 1200, 1200, true);
    assert_eq!(resized.width(), 1200);
    assert_eq!(resized.height(), 1200);

    // Really wide
    let resized = resize_preserving_aspect(&image, 1200, 100, true);
    assert_eq!(resized.width(), 1200);
    assert_eq!(resized.height(), 100);

    // Really tall
    let resized = resize_preserving_aspect(&image, 100, 1200, true);
    assert_eq!(resized.width(), 100);
    assert_eq!(resized.height(), 1200);
  }

  #[test]
  fn exact_fit_of_extreme_aspect_source_has_exact_size() {
    let sliver = DynamicImage::new_rgb8(4000, 1);
    let resized = resize_preserving_aspect(&sliver, 64, 64, true);
    assert_eq!((resized.width(), resized.height()), (64, 64));
  }
}
