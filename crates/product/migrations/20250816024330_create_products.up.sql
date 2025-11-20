-- Add up migration script here
CREATE TABLE IF NOT EXISTS products (
    product_id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    price BIGINT NOT NULL,
    stock INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP NULL
);

-- Indeks
CREATE INDEX idx_products_name ON products (name);