-- noinspection SqlDialectInspectionForFile
-- noinspection SqlNoDataSourceInspectionForFile
-- noinspection SqlResolveForFile

-- One approval request for a new cookie-based user session on another device.
-- See _database/sql/user_login_challenges.md for transitions and API requirements.
-- This table does not authorize MCP sessions or API keys.
CREATE TABLE user_login_challenges (
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,

  -- Non-secret identifier for logs/support. Never sufficient to approve or redeem.
  token VARCHAR(32) NOT NULL,

  -- SHA-256 digests of TWO INDEPENDENT cryptographically random 256-bit tokens.
  -- Hash the canonical unpadded base64url token text; never store the raw tokens.
  -- The approval token goes in the browser/QR URL and only permits reviewing a
  -- request and submitting an authenticated user's explicit approve/decline choice.
  approval_token_sha256 BINARY(32) NOT NULL,

  -- Only the initiating device receives this token. Required for polling/redeeming.
  -- Never include it in the approval URL, QR code, browser page, or application logs.
  device_token_sha256 BINARY(32) NOT NULL,

  -- Eight random uppercase consonants, displayed as e.g. WDJB-MJHT on BOTH screens.
  -- Comparison aid only: not unique, not a lookup key, not an authentication factor.
  confirmation_code CHAR(8) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,

  -- UserLoginChallengeStatus: pending, approved, redeemed, failed.
  -- Only pending may be approved/declined. Only approved may mint a session.
  -- Expiry is checked independently of status on EVERY approval/redemption request.
  -- Application inserts explicitly bind UserLoginChallengeStatus::Pending.
  status VARCHAR(16) NOT NULL,

  -- The authenticated web user who approved OR declined. NULL until a decision.
  -- On approval this becomes the immutable owner of the session to be minted.
  maybe_deciding_user_token VARCHAR(32) DEFAULT NULL,

  -- Set atomically with status=redeemed and session insertion, under a row lock.
  -- Numeric reference avoids copying a session credential into this audit table.
  -- Intentionally signed to match the existing user_sessions.id column.
  -- At most one session per challenge; retries may only return that SAME live session.
  maybe_redeemed_user_session_id BIGINT(20) DEFAULT NULL,

  -- UserLoginChallengeFailureType: user_declined, expired.
  -- Only failed rows have a failure type and maybe_failed_at.
  -- Transient errors / invalid credentials must not overwrite a valid challenge.
  maybe_failure_type VARCHAR(32) DEFAULT NULL,

  -- ========== IP ADDRESSES ==========

  -- Server-observed requesting device IP, displayed on the approval screen.
  -- Canonical IPv4/IPv6 strings; never accept an IP supplied in a request body.
  ip_address_creation VARCHAR(40) NOT NULL,

  -- Browser/mobile IP for approval or denial; may differ from the device IP.
  maybe_ip_address_decision VARCHAR(40) DEFAULT NULL,

  -- Device IP at FIRST successful redemption. Retries do not overwrite it.
  maybe_ip_address_redemption VARCHAR(40) DEFAULT NULL,

  -- For user_declined this is the browser IP. For expiry there is no actor/IP.
  -- ip_address_creation is retained on ALL failures, including timeouts.
  maybe_ip_address_failure VARCHAR(40) DEFAULT NULL,

  -- ========== TIMESTAMPS ==========

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  -- Immutable deadline: insert with created_at=NOW(), expires_at=NOW()+INTERVAL 20 MINUTE.
  -- The default deliberately fails closed if a writer forgets to set the deadline.
  -- Approval, polling, and retrying must NEVER extend it. NOW() >= expires_at is expired.
  expires_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  maybe_decided_at TIMESTAMP NULL DEFAULT NULL,
  maybe_redeemed_at TIMESTAMP NULL DEFAULT NULL,

  -- Decision time for user_declined; expires_at for expired (even if recorded later).
  maybe_failed_at TIMESTAMP NULL DEFAULT NULL,

  -- INDICES --
  PRIMARY KEY (id),
  UNIQUE KEY (token),
  UNIQUE KEY (approval_token_sha256),
  UNIQUE KEY (device_token_sha256),
  UNIQUE KEY (maybe_redeemed_user_session_id),
  KEY index_status_expires_at (status, expires_at),
  KEY index_expires_at (expires_at),
  KEY index_deciding_user_created_at (maybe_deciding_user_token, created_at),
  KEY index_creation_ip_created_at (ip_address_creation, created_at),
  KEY index_failure_ip_failed_at (maybe_ip_address_failure, maybe_failed_at)

) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
