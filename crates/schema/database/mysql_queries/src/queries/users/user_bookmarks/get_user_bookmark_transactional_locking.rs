use anyhow::anyhow;
use chrono::{DateTime, Utc};
use sqlx::{Error, Executor, MySql};

use enums::by_table::user_bookmarks::user_bookmark_entity_type::UserBookmarkEntityType;
use errors::AnyhowResult;
use tokens::tokens::user_bookmarks::UserBookmarkToken;
use tokens::tokens::users::UserToken;

use crate::queries::users::user_bookmarks::user_bookmark_entity_token::UserBookmarkEntityToken;

pub struct UserBookmark {
  pub token: UserBookmarkToken,

  pub entity_type: UserBookmarkEntityType,
  pub entity_token: String,

  pub user_token: UserToken,

  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub maybe_deleted_at: Option<DateTime<Utc>>,
}

pub enum BookmarkIdentifier<'a> {
  BookmarkToken(&'a UserBookmarkToken),
  /// The given user's bookmark of an entity.
  UserAndEntity(&'a UserToken, &'a UserBookmarkEntityToken),
}

/// Use SELECT FOR UPDATE to lock the rows for the duration of the transaction.
/// We're locking two tables at once: https://stackoverflow.com/a/38545378
pub async fn get_user_bookmark_transactional_locking<'e, 'c, E>(
    bookmark_identifier: BookmarkIdentifier<'e>,
    mysql_executor: E
) -> AnyhowResult<Option<UserBookmark>>
  where E: 'e + Executor<'c, Database = MySql>
{
  let maybe_results = match bookmark_identifier {
    BookmarkIdentifier::UserAndEntity(user_token, bookmark_entity_token) => {
      query_by_user_and_entity(QueryByUserAndEntityArgs {
        user_token,
        bookmark_entity_token,
        mysql_executor,
      }).await
    }
    BookmarkIdentifier::BookmarkToken(bookmark_token) => {
      query_by_bookmark_token(mysql_executor, bookmark_token).await
    }
  };

  match maybe_results {
    Ok(user_bookmark) => Ok(Some(UserBookmark {
      token: user_bookmark.token,
      entity_type: user_bookmark.entity_type,
      entity_token: user_bookmark.entity_token,
      user_token: user_bookmark.user_token,
      created_at: user_bookmark.created_at,
      updated_at: user_bookmark.updated_at,
      maybe_deleted_at: user_bookmark.deleted_at,
    })),
    Err(err) => match err {
      sqlx::Error::RowNotFound => Ok(None),
      _ => Err(anyhow!("Error querying for IP ban: {:?}", err)),
    }
  }
}

async fn query_by_user_and_entity<'c, E>(args: QueryByUserAndEntityArgs<'_, E>)
  -> Result<RawUserBookmark, Error> where E: Executor<'c, Database=MySql>
{
  let QueryByUserAndEntityArgs { user_token, bookmark_entity_token, mysql_executor } = args;
  let (entity_type, entity_token) = bookmark_entity_token.get_composite_keys();

  // NB(1): LEFT OUTER JOIN does not require entity_stats to be present, but will lock it under
  // SELECT...FOR UPDATE if the row exists.
  // NB(2): This locks every user's bookmark of the entity, so that bookmark changes to the same
  // entity stay serialized. Sorting on an expression makes MySQL read (and lock) all of them
  // before LIMIT 1 returns the requesting user's bookmark first. If the user has no bookmark,
  // the row returned belongs to someone else and is discarded below.
  let maybe_bookmark = sqlx::query_as!(
        RawUserBookmark,
          r#"
  SELECT
      b.token as `token: tokens::tokens::user_bookmarks::UserBookmarkToken`,

      b.entity_type as `entity_type: enums::by_table::user_bookmarks::user_bookmark_entity_type::UserBookmarkEntityType`,
      b.entity_token,

      b.user_token as `user_token: tokens::tokens::users::UserToken`,

      es.bookmark_count as unused_bookmark_count,

      b.created_at,
      b.updated_at,
      b.deleted_at

  FROM
      user_bookmarks AS b

  LEFT OUTER JOIN entity_stats AS es
      ON b.entity_type = es.entity_type
      AND b.entity_token = es.entity_token
  WHERE
      b.entity_type = ?
      AND b.entity_token = ?
  ORDER BY
      (b.user_token = ?) DESC
  LIMIT 1
  FOR UPDATE
          "#,
        entity_type.to_str(),
        entity_token,
        user_token
      )
      .fetch_optional(mysql_executor)
      .await?;

  maybe_bookmark
      .filter(|bookmark| bookmark.user_token == *user_token)
      .ok_or(Error::RowNotFound)
}

async fn query_by_bookmark_token<'e, 'c, E>(mysql_executor: E, bookmark_token: &UserBookmarkToken)
  -> Result<RawUserBookmark, Error> where E: 'e + Executor<'c, Database=MySql>
{
  // NB(1): LEFT OUTER JOIN does not require entity_stats to be present, but will lock it under
  // SELECT...FOR UPDATE if the row exists.
  // NB(2): We're joining the record against itself to lock on (entity_type,entity_token).
  // The selected columns must come from b1, the requested bookmark; b2 ranges over every
  // bookmark of the same entity.
  sqlx::query_as!(
        RawUserBookmark,
          r#"
  SELECT
      b1.token as `token: tokens::tokens::user_bookmarks::UserBookmarkToken`,

      b1.entity_type as `entity_type: enums::by_table::user_bookmarks::user_bookmark_entity_type::UserBookmarkEntityType`,
      b1.entity_token,

      b1.user_token as `user_token: tokens::tokens::users::UserToken`,

      es.bookmark_count as unused_bookmark_count,

      b1.created_at,
      b1.updated_at,
      b1.deleted_at

  FROM
      user_bookmarks AS b1

  JOIN user_bookmarks as b2
      ON b1.entity_type = b2.entity_type
      AND b1.entity_token = b2.entity_token

  LEFT OUTER JOIN entity_stats AS es
      ON b1.entity_type = es.entity_type
      AND b1.entity_token = es.entity_token

  WHERE
      b1.token = ?

  FOR UPDATE
          "#,
        bookmark_token
      )
      .fetch_one(mysql_executor)
      .await
}

struct QueryByUserAndEntityArgs<'a, E> {
  user_token: &'a UserToken,
  bookmark_entity_token: &'a UserBookmarkEntityToken,
  mysql_executor: E,
}

pub struct RawUserBookmark {
  pub token: UserBookmarkToken,

  pub entity_type: UserBookmarkEntityType,
  pub entity_token: String,

  pub user_token: UserToken,

  // NB: Not used, just for dual transactional locking.
  pub unused_bookmark_count: Option<u32>,

  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub deleted_at: Option<DateTime<Utc>>,
}
