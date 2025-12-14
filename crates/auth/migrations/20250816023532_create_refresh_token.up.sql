-- Add up migration script here
CREATE TABLE refresh_tokens (
    refresh_token_id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    token VARCHAR(255) NOT NULL UNIQUE,
    expiration TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT current_timestamp,
    updated_at TIMESTAMP DEFAULT current_timestamp,
    deleted_at TIMESTAMP DEFAULT NULL
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_active_token ON refresh_tokens (token)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_active_user ON refresh_tokens (user_id)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_active_expiration ON refresh_tokens (expiration)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_deleted_at ON refresh_tokens (deleted_at)
WHERE
    deleted_at IS NOT NULL;