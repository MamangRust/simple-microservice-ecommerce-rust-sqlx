-- Add up migration script here
CREATE TABLE "reset_tokens" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INT NOT NULL UNIQUE,
    "token" TEXT NOT NULL UNIQUE,
    "expiry_date" TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reset_tokens_token ON reset_tokens (token);

CREATE INDEX IF NOT EXISTS idx_reset_tokens_expiry ON reset_tokens (expiry_date);