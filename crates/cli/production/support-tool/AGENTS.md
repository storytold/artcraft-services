# support-tool

Production support CLI for debugging and manual operations against external services.

## Building & Running

```bash
cargo build -p support-tool
cargo run -p support-tool -- kinovi_web generate_video --prompt "A corgi at the lake"
```

## Environment

Requires a `.env-support-tool-secrets` file (or env vars) with:

- `SEEDANCE2PRO_VOLCENGINE_COOKIES` — Volcengine cookies (legacy fallback: `SEEDANCE2PRO_COOKIES`)
- `SEEDANCE2PRO_BYTEPLUS_COOKIES` — BytePlus cookies (legacy fallback: `SEEDANCE2PRO_ALT_COOKIES`)
- `SEEDANCE2PRO_BYTEPLUS_ULTRA_COOKIES` — BytePlus Ultra cookies
- `ARTCRAFT_COOKIES` — session cookies for ArtCraft API auth (format: `session=...; visitor=...`)

All `kinovi_web` commands (also available as `kinovi` / `seedance2pro`) accept
`--kinovi-account volcengine|byteplus|byteplusultra`, defaulting to `volcengine`.
Use `--cookies-env ENV_VAR_NAME` to explicitly override that account's cookie
configuration. The override has no default; when omitted, only the selected
account's variables and its legacy fallback are checked. Either flag can appear
before or after the subcommand. CSV account labels default to the resolved cookie
env var name; `--account` still overrides the output label.

```bash
support-tool kinovi_web account_info --kinovi-account byteplus
support-tool seedance2pro find_job --token <order_id> --kinovi-account byteplusultra
support-tool kinovi_web account_info --kinovi-account byteplus --cookies-env CUSTOM_COOKIES
```

## Architecture

- Entry point: `src/main.rs` → normalizes args (underscores optional) → dispatches
- Top-level commands in `src/commands/run.rs` (`TopLevelCommand` enum)
- Each top-level command has its own module with `dispatch.rs`, `state.rs`, `subcommands/`
- Subcommands use `#[derive(Args)]` for their argument structs
- All subcommand `run()` functions are `async fn` returning `anyhow::Result<()>`

## Adding a New Subcommand (to kinovi_web)

1. Create `src/commands/kinovi_web/subcommands/my_command.rs`
2. Add `pub mod my_command;` to `subcommands/mod.rs`
3. Add the name to `SUBCOMMAND_NAMES` in `dispatch.rs`
4. Add a variant to `KinoviWebCommand` enum in `dispatch.rs`
5. Add a match arm in `dispatch.rs::run()`

## Utilities

- `src/utils/parse_video_model.rs` — shared parser for loose video model names → `CommonVideoModel`
- `src/utils/normalize_subcommands.rs` — underscore-insensitive arg normalizer

## Conventions

- Use `log::info!()` for status output, never `println!` (except for final results)
- Use `anyhow::anyhow!()` for ad-hoc errors
- Get `KinoviWebSession` from `state.cookies` via `from_cookies_string()`
- Get `StorytellerCredentialSet` from `state.creds` (parsed from ARTCRAFT_COOKIES)
- External crates used: `artcraft_client` (media file download, omni API), `kinovi_web_client` (Kinovi direct), `artcraft_api_defs` (request types)

## Subcommands

### kinovi_web (direct Kinovi API)

- `kinovi_web find_job --token <order_id>` — search for a job across all pages
- `kinovi_web failed_job_histogram` — histogram of failure reasons
- `kinovi_web generate_video --prompt <text_or_file> [--model seedance2p0] [--start_frame_media_token mf_xxx] [--localhost] [--download_path /tmp/media_files]`

### artcraft (omni API)

- `artcraft generate_video --prompt <text_or_file> [--model seedance2p0] [--start_frame_media_token mf_xxx] [--duration 5] [--localhost]`
