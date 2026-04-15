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
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
) WITHOUT ROWID;

CREATE UNIQUE INDEX idx_user_account_profile_username
ON user_account_profile(username);

DROP TRIGGER IF EXISTS trg_user_account_profile_username_insert_guard;
CREATE TRIGGER trg_user_account_profile_username_insert_guard
BEFORE INSERT ON user_account_profile
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.username IS NULL OR trim(NEW.username) = '' THEN
            RAISE(ABORT, 'username required')
    END;
    SELECT CASE
        WHEN length(trim(NEW.username)) < 3 OR length(trim(NEW.username)) > 32 THEN
            RAISE(ABORT, 'username length invalid')
    END;
    SELECT CASE
        WHEN trim(NEW.username) GLOB '*[^a-z0-9-]*' THEN
            RAISE(ABORT, 'username characters invalid')
    END;
    SELECT CASE
        WHEN NEW.username != lower(trim(NEW.username)) THEN
            RAISE(ABORT, 'username must be lowercase')
    END;
END;

DROP TRIGGER IF EXISTS trg_user_account_profile_username_update_guard;
CREATE TRIGGER trg_user_account_profile_username_update_guard
BEFORE UPDATE OF username ON user_account_profile
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.username IS NULL OR trim(NEW.username) = '' THEN
            RAISE(ABORT, 'username required')
    END;
    SELECT CASE
        WHEN length(trim(NEW.username)) < 3 OR length(trim(NEW.username)) > 32 THEN
            RAISE(ABORT, 'username length invalid')
    END;
    SELECT CASE
        WHEN trim(NEW.username) GLOB '*[^a-z0-9-]*' THEN
            RAISE(ABORT, 'username characters invalid')
    END;
    SELECT CASE
        WHEN NEW.username != lower(trim(NEW.username)) THEN
            RAISE(ABORT, 'username must be lowercase')
    END;
END;
