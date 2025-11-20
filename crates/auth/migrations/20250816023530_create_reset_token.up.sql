-- Add up migration script here
CREATE TABLE "reset_tokens" (
    "id" SERIAL PRIMARY KEY,
    "user_id" INT NOT NULL UNIQUE,
    "token" TEXT NOT NULL UNIQUE,
    "expiry_date" TIMESTAMP NOT NULL
);

CREATE INDEX idx_reset_token_token ON reset_tokens (token);

CREATE INDEX idx_reset_token_user_id ON reset_tokens (user_id);