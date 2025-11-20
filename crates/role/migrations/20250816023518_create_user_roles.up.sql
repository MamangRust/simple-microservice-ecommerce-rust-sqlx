-- Add up migration script here
CREATE TABLE IF NOT EXISTS user_roles (
    user_role_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_user_roles_user_id ON user_roles (user_id);

CREATE INDEX IF NOT EXISTS idx_user_roles_role_id ON user_roles (role_id);

CREATE INDEX IF NOT EXISTS idx_user_roles_user_id_role_id ON user_roles (user_id, role_id);

CREATE INDEX IF NOT EXISTS idx_user_roles_created_at ON user_roles (created_at);

CREATE INDEX IF NOT EXISTS idx_user_roles_updated_at ON user_roles (updated_at);

CREATE INDEX IF NOT EXISTS idx_user_roles_deleted_at ON user_roles (deleted_at);