-- Add up migration script here
CREATE TABLE IF NOT EXISTS roles (
    role_id SERIAL PRIMARY KEY,
    role_name VARCHAR(50) NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP NULL
);

-- Indeks
CREATE INDEX IF NOT EXISTS idx_roles_active_name ON roles (role_name)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_roles_active_created_at ON roles (created_at DESC)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_roles_trashed_created_at ON roles (created_at DESC)
WHERE
    deleted_at IS NOT NULL;