use enums::common::visibility::Visibility;
use tokens::tokens::users::UserToken;

/// Whether `requester` may attach an image as the cover of a media file or model weight.
///
/// A cover is shown to everyone who can see the record it decorates, so attaching an image
/// discloses it. Public and hidden images may be used by anyone holding the token; a private
/// image only by the user who created it (moderators included — moderation rights don't extend
/// to publishing another user's private image).
pub fn may_use_as_cover_image(
  cover_visibility: Visibility,
  maybe_cover_creator_user_token: Option<&UserToken>,
  requester_user_token: &UserToken,
) -> bool {
  if cover_visibility != Visibility::Private {
    return true;
  }
  maybe_cover_creator_user_token
      .is_some_and(|creator| creator.as_str() == requester_user_token.as_str())
}

#[cfg(test)]
mod tests {
  use enums::common::visibility::Visibility;
  use tokens::tokens::users::UserToken;

  use super::may_use_as_cover_image;

  const OWNER: &str = "U:OWNER";
  const OTHER: &str = "U:OTHER";

  #[test]
  fn public_and_hidden_images_are_usable_by_anyone() {
    let owner = UserToken::new_from_str(OWNER);
    let other = UserToken::new_from_str(OTHER);
    assert!(may_use_as_cover_image(Visibility::Public, Some(&owner), &other));
    assert!(may_use_as_cover_image(Visibility::Hidden, Some(&owner), &other));
  }

  #[test]
  fn private_image_is_usable_only_by_its_creator() {
    let owner = UserToken::new_from_str(OWNER);
    let other = UserToken::new_from_str(OTHER);
    assert!(may_use_as_cover_image(Visibility::Private, Some(&owner), &owner));
    assert!(!may_use_as_cover_image(Visibility::Private, Some(&owner), &other));
    assert!(!may_use_as_cover_image(Visibility::Private, None, &other));
  }
}
