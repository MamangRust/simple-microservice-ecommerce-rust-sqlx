-- Add up migration script here
CREATE TABLE IF NOT EXISTS orders (
    order_id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    total_price BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL
);

-- Index
CREATE INDEX IF NOT EXISTS idx_orders_user_active_created_at ON orders (user_id, created_at DESC)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_orders_active_created_at ON orders (created_at DESC)
WHERE
    deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_orders_trashed_created_at ON orders (created_at DESC)
WHERE
    deleted_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_orders_active_total_price ON orders (total_price)
WHERE
    deleted_at IS NULL;