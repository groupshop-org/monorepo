PRAGMA foreign_keys = ON;

CREATE TABLE user_account_profile (
    user_id TEXT PRIMARY KEY,
    username TEXT NOT NULL COLLATE NOCASE,
    full_name TEXT NOT NULL DEFAULT '',
    shipping_line1 TEXT NOT NULL DEFAULT '',
    shipping_line2 TEXT NOT NULL DEFAULT '',
    shipping_city TEXT NOT NULL DEFAULT '',
    shipping_state TEXT NOT NULL DEFAULT '',
    shipping_postal_code TEXT NOT NULL DEFAULT '',
    shipping_country TEXT NOT NULL DEFAULT '',
    receive_marketing INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (trim(username) != ''),
    CHECK (length(trim(username)) BETWEEN 3 AND 32),
    CHECK (trim(username) NOT GLOB '*[^a-z0-9-]*'),
    CHECK (username = lower(trim(username)))
) WITHOUT ROWID;

CREATE UNIQUE INDEX idx_user_account_profile_username
ON user_account_profile(username);
