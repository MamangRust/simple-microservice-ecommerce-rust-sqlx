-- Add up migration script here
CREATE TABLE "orders" (
    "order_id" SERIAL PRIMARY KEY,
    "user_id" INT NOT NULL,
    "total_price" INT NOT NULL,
    "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "deleted_at" TIMESTAMP DEFAULT NULL
);

CREATE INDEX "idx_orders_user_id" ON "orders" ("user_id");

CREATE INDEX "idx_orders_total_price" ON "orders" ("total_price");