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
CREATE INDEX IF NOT EXISTS idx_users_email ON users (email);

CREATE INDEX IF NOT EXISTS idx_users_firstname ON users (firstname);

CREATE INDEX IF NOT EXISTS idx_users_lastname ON users (lastname);

CREATE INDEX IF NOT EXISTS idx_users_firstname_lastname ON users (firstname, lastname);

CREATE INDEX IF NOT EXISTS idx_users_created_at ON users (created_at);
