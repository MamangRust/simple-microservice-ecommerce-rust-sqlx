-- Add up migration script here
CREATE TABLE IF NOT EXISTS products (
    product_id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    price BIGINT NOT NULL,
    stock INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL
);

-- Index
CREATE INDEX IF NOT EXISTS idx_products_active_created_at ON products (created_at DESC)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_products_trashed_created_at ON products (created_at DESC)
WHERE
    deleted_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_products_active_id ON products (product_id DESC)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_products_name ON products (name);