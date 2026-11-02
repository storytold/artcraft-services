-- Materialized from 2026-11-03-000000-0000_create_oauth_authorization.
-- Secrets are SHA-256 hex. The raw code, access token, refresh token, and consent nonce are not stored.

CREATE TABLE oauth_clients (
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,
  token VARCHAR(64) NOT NULL,
  client_name VARCHAR(80) NOT NULL,
  owner_user_token VARCHAR(64) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  maybe_deleted_at TIMESTAMP NULL,
  PRIMARY KEY (id),
  UNIQUE KEY (token),
  KEY index_owner_user_token (owner_user_token)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE oauth_client_redirects (
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,
  client_token VARCHAR(64) NOT NULL,
  redirect_uri VARCHAR(2048) NOT NULL,
  redirect_hash CHAR(64) NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY unique_client_redirect (client_token, redirect_hash),
  KEY index_client_token (client_token)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE oauth_consents (
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,
  user_token VARCHAR(64) NOT NULL,
  client_token VARCHAR(64) NOT NULL,
  scopes VARCHAR(255) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  maybe_revoked_at TIMESTAMP NULL,
  PRIMARY KEY (id),
  UNIQUE KEY unique_user_client (user_token, client_token),
  KEY index_client_token (client_token)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE oauth_authorization_codes (
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,
  code_hash CHAR(64) NOT NULL,
  client_token VARCHAR(64) NOT NULL,
  user_token VARCHAR(64) NOT NULL,
  redirect_uri VARCHAR(2048) NOT NULL,
  scopes VARCHAR(255) NOT NULL,
  code_challenge VARCHAR(128) NOT NULL,
  audience VARCHAR(255) NOT NULL,
  expires_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  maybe_used_at TIMESTAMP NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (id),
  UNIQUE KEY (code_hash),
  KEY index_client_token (client_token)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE oauth_consent_nonces (
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,
  nonce_hash CHAR(64) NOT NULL,
  user_token VARCHAR(64) NOT NULL,
  client_token VARCHAR(64) NOT NULL,
  request_hash CHAR(64) NOT NULL,
  expires_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  maybe_used_at TIMESTAMP NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (id),
  UNIQUE KEY (nonce_hash)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE oauth_tokens (
  id BIGINT(20) UNSIGNED NOT NULL AUTO_INCREMENT,
  access_hash CHAR(64) NOT NULL,
  refresh_hash CHAR(64) NOT NULL,
  maybe_previous_refresh_hash CHAR(64) DEFAULT NULL,
  client_token VARCHAR(64) NOT NULL,
  user_token VARCHAR(64) NOT NULL,
  scopes VARCHAR(255) NOT NULL,
  audience VARCHAR(255) NOT NULL,
  access_expires_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  refresh_expires_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  maybe_revoked_at TIMESTAMP NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (id),
  UNIQUE KEY (access_hash),
  UNIQUE KEY (refresh_hash),
  KEY index_previous_refresh (maybe_previous_refresh_hash),
  KEY index_user_client (user_token, client_token)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
