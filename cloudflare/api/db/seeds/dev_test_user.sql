-- Deterministic local-only test user for auth/admin development.

PRAGMA foreign_keys = ON;

INSERT OR IGNORE INTO user_account_profile (
    user_id,
    username,
    receive_marketing,
    created_at,
    updated_at
) VALUES (
    'O1G1est5Iq21uhfH6IUyMj_aqpJuGgKd0npy3tXYRUk',
    'dakom',
    0,
    '2026-04-16 00:00:00',
    '2026-04-16 00:00:00'
);

-- Matching local credential for the deterministic test user above.
INSERT OR IGNORE INTO user_auth_email (
    email,
    password_hash,
    user_id,
    created_at
) VALUES (
    'david.komer@gmail.com',
    '$argon2id$v=19$m=19456,t=2,p=1$NAoOntonakZqu3z2YtBztY8cGrg$Qur38bMGdc6TsdEs5LAa5E5AC+bp6KEcVC+vhU47g20',
    'O1G1est5Iq21uhfH6IUyMj_aqpJuGgKd0npy3tXYRUk',
    '2026-04-16 00:00:00'
);

-- Grants the local test user the full role set needed for admin/dev workflows.
INSERT OR IGNORE INTO user_auth_role (user_id, role_id) VALUES
    ('O1G1est5Iq21uhfH6IUyMj_aqpJuGgKd0npy3tXYRUk', 1),
    ('O1G1est5Iq21uhfH6IUyMj_aqpJuGgKd0npy3tXYRUk', 2),
    ('O1G1est5Iq21uhfH6IUyMj_aqpJuGgKd0npy3tXYRUk', 3),
    ('O1G1est5Iq21uhfH6IUyMj_aqpJuGgKd0npy3tXYRUk', 4);
