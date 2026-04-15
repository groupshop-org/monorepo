PRAGMA foreign_keys = ON;

CREATE TABLE user_auth_email (
    email TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
) WITHOUT ROWID;

CREATE INDEX idx_user_auth_email_user_id ON user_auth_email(user_id);

CREATE TABLE user_auth_openid (
    provider TEXT NOT NULL,
    subject TEXT NOT NULL,
    email TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (provider, subject)
) WITHOUT ROWID;

CREATE INDEX idx_user_auth_openid_user_id ON user_auth_openid(user_id);
CREATE INDEX idx_user_auth_openid_email ON user_auth_openid(email);

CREATE TABLE user_role_info (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT
);

CREATE TABLE user_auth_role (
    user_id TEXT NOT NULL,
    role_id INTEGER NOT NULL,
    PRIMARY KEY (user_id, role_id),
    FOREIGN KEY (role_id) REFERENCES user_role_info(id)
);

INSERT OR IGNORE INTO user_role_info (id, name, description) VALUES
    (1, 'partial_registration', 'Partial registration'),
    (2, 'email_verified', 'Email verified'),
    (3, 'username_chosen', 'Username has been chosen'),
    (4, 'admin', 'Administrator');
