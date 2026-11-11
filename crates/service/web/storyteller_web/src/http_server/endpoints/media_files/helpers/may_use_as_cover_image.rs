use enums::common::visibility::Visibility;
use tokens::tokens::users::UserToken;

/// Whether `requester` may attach an image as the cover of a media file or model weight.
///
/// A cover is shown to everyone who can see the record it decorates, so attaching an image
/// discloses it. Public and hidden images may be used by anyone holding the token; a private
/// image only by the user who created it (moderators included — moderation rights don't extend
/// to publishing another user's private image).
///
/// When `maybe_required_creator_user_token` is `Some` (the record's own creator, which may itself be
/// `None` for a creator-less record), a private image must also be that user's: reads only show a private cover that shares the record's creator, so any
/// other pairing would be accepted here and then silently hidden.
pub fn may_use_as_cover_image(
  cover_visibility: Visibility,
  maybe_cover_creator_user_token: Option<&UserToken>,
  requester_user_token: &UserToken,
  maybe_required_creator_user_token: Option<Option<&UserToken>>,
) -> bool {
  if cover_visibility != Visibility::Private {
    return true;
  }
  maybe_cover_creator_user_token.is_some_and(|creator| {
    creator.as_str() == requester_user_token.as_str()
        && maybe_required_creator_user_token
            .map_or(true, |required| required.is_some_and(|required| creator.as_str() == required.as_str()))
  })
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
    assert!(may_use_as_cover_image(Visibility::Public, Some(&owner), &other, None));
    assert!(may_use_as_cover_image(Visibility::Hidden, Some(&owner), &other, None));
  }

  #[test]
  fn private_image_is_usable_only_by_its_creator() {
    let owner = UserToken::new_from_str(OWNER);
    let other = UserToken::new_from_str(OTHER);
    assert!(may_use_as_cover_image(Visibility::Private, Some(&owner), &owner, None));
    assert!(!may_use_as_cover_image(Visibility::Private, Some(&owner), &other, None));
    assert!(!may_use_as_cover_image(Visibility::Private, None, &other, None));
  }

  #[test]
  fn private_image_must_share_the_records_creator_when_one_is_required() {
    let owner = UserToken::new_from_str(OWNER);
    let other = UserToken::new_from_str(OTHER);
    assert!(may_use_as_cover_image(Visibility::Private, Some(&owner), &owner, Some(Some(&owner))));
    // A moderator's own private image on someone else's record would be hidden on read.
    assert!(!may_use_as_cover_image(Visibility::Private, Some(&other), &other, Some(Some(&owner))));
    assert!(may_use_as_cover_image(Visibility::Public, Some(&other), &other, Some(Some(&owner))));
    // A creator-less record can never show a private cover.
    assert!(!may_use_as_cover_image(Visibility::Private, Some(&other), &other, Some(None)));
  }
}
