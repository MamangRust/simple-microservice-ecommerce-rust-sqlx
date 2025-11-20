-- Add up migration script here
CREATE TABLE IF NOT EXISTS roles (
    role_id SERIAL PRIMARY KEY,
    role_name VARCHAR(50) NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP NULL
);

-- Indeks
CREATE INDEX IF NOT EXISTS idx_roles_role_name ON roles (role_name);

CREATE INDEX IF NOT EXISTS idx_roles_created_at ON roles (created_at);

CREATE INDEX IF NOT EXISTS idx_roles_updated_at ON roles (updated_at);
