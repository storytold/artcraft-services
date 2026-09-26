-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

-- UserWebSessionCreationType: direct_login, device_approval, impersonation.
-- NULL means unknown / legacy writer, not direct_login. Do not backfill guesses.
-- Additive metadata only: does not change validation, expiry, or impersonation rules.
-- MCP sessions and API keys are separate tables and do not use this field.
ALTER TABLE user_sessions
  ADD COLUMN maybe_creation_type VARCHAR(32) DEFAULT NULL;
