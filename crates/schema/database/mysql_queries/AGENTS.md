# mysql_queries

SQLx-based MySQL queries for the storyteller-web backend and background jobs.

## Building

```
SQLX_OFFLINE=true cargo check -p mysql_queries
```

The `SQLX_OFFLINE=true` env var is **required** for compilation. This crate uses the sqlx
offline cache (`.sqlx/` directory at the workspace root) to validate queries at compile time
without a live database connection.

## Query Style

### Prefer `sqlx::query!` and `sqlx::query_as!` macros

Always use the compile-time checked `sqlx::query!` and `sqlx::query_as!` macros over the
runtime `sqlx::query()` and `sqlx::query_as()` functions. The macros validate SQL syntax,
column names, and parameter types against the offline cache at compile time.

```rust
// PREFERRED: compile-time checked
sqlx::query!(
  r#"INSERT INTO my_table SET token = ?, name = ?"#,
  token,
  name,
)
  .execute(args.mysql_executor)
  .await?;

// AVOID: runtime only, no compile-time checking
sqlx::query(r#"INSERT INTO my_table SET token = ?, name = ?"#)
  .bind(token)
  .bind(name)
  .execute(args.mysql_executor)
  .await?;
```

Use runtime `sqlx::query()` only when:
- The query involves a table not yet in the offline cache and you cannot run `cargo sqlx prepare`
- The query is dynamically constructed (rare)
- SELECT queries for new tables where column type metadata isn't cached yet

### Offline Cache

When adding new queries for **existing tables**, the `.sqlx/` cache should already have the
schema metadata. For **new tables**, you may need to:

1. Run the migration against a real database
2. Run `cargo sqlx prepare` to regenerate the cache
3. Or manually create cache entries for simple INSERT/UPDATE queries (they only need parameter count)

### Table Conventions

- Primary key `id` columns must be `BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT`. Do not use signed IDs.
- Always create tables with `ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin`.
- Do not give application-owned state/status/type columns schema defaults. Keep required
  columns `NOT NULL` without a `DEFAULT`; application code must explicitly bind their
  initial value on every insert. Keep migrations and materialized schema docs in sync.

### Database enum values

- Use the Rust enums in `enums::by_table` for database state, status, failure, and type
  values. Do not hardcode their string representations in SQL or Rust business logic.
- Bind enum values (or their `to_str()` representation) through SQL parameters, including
  `SET`, `WHERE`, `IN`, and `INSERT` values. For example, bind
  `UserLoginChallengeStatus::Approved.to_str()` to `status = ?`.
- Query args and returned rows must use the enum type rather than `String`/`&str` for
  enum columns. Use SQLx column type overrides and the enum's MySQL decoder on reads;
  use exhaustive variant matches in application logic. Preserve existing stored values.

### Error Types

New queries should return `Result<T, sqlx::Error>` instead of the legacy `AnyhowResult<T>`.
The `AnyhowResult` pattern is being phased out.

### Named arguments and generic transactors

Every new or refactored query must take a single named `<QueryName>Args` struct containing
all inputs, including `mysql_executor`. Do not use positional parameter lists, even for
small queries. Keep the args struct in the same file as its query.

Use a generic transactor `T` bounded by `Executor<'c, Database = MySql>`. Do not constrain
query functions to `MySqlConnection`, `MySqlPool`, a concrete transaction, or the legacy
`Transactor` enum. This applies to all queries, not only ones currently used in transactions.
Put the executor bound on the function; the args struct need not carry executor lifetimes
or `PhantomData` when it simply stores `T`.

```rust
use sqlx::{Executor, MySql};

pub struct MyQueryArgs<'a, T> {
  pub field: &'a str,
  pub mysql_executor: T,
}

pub async fn my_query<'c, T>(args: MyQueryArgs<'_, T>) -> Result<(), sqlx::Error>
where
  T: Executor<'c, Database = MySql>,
{
  sqlx::query!("UPDATE my_table SET name = ?", args.field)
    .execute(args.mysql_executor)
    .await?;
  Ok(())
}
```

Call with `mysql_executor: &pool` for a pool, or `mysql_executor: &mut *tx` /
`mysql_executor: &mut *conn` to reborrow the connection held by an owned SQLx transaction /
pool connection. SQLx transactions do not themselves implement `Executor`; dereference
them at the call site. Reuse the caller's transaction when atomicity or row locks matter.
Do not acquire a new connection or begin/commit a transaction inside a query helper.

### One query per file, grouped by table

- Put each query in its own descriptively named `.rs` file under the table's directory
  module, e.g. `queries/user_login_challenges/decide_challenge.rs` or the existing
  `queries/users/user_sessions/find_redeemed_session.rs`.
- Each query file contains one SQL statement/query function, its args struct, and any
  result type used only by that query. Put shared result types in a separate type file.
- Keep `mod.rs` for module declarations; do not collect SQL in it or in catch-all files
  such as `challenge_queries.rs` or `session_queries.rs`.
- Split multi-query operations into individual query files. Keep transaction sequencing
  and business decisions in the caller/service layer, with every statement using the
  same transaction when required.
