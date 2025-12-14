-- Add up migration script here
CREATE TABLE IF NOT EXISTS users (
    user_id SERIAL PRIMARY KEY,
    firstname VARCHAR(100) NOT NULL,
    lastname VARCHAR(100) NOT NULL,
    email VARCHAR(100) NOT NULL UNIQUE,
    password VARCHAR(100) NOT NULL,
    verification_code VARCHAR(100) NOT NULL,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP NULL
);

-- Indeks
CREATE INDEX IF NOT EXISTS idx_users_active_email ON users (email)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_users_active_created_at ON users (created_at DESC)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_users_active_firstname_lastname ON users (firstname, lastname)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_users_trashed_created_at ON users (created_at DESC)
WHERE
    deleted_at IS NOT NULL;