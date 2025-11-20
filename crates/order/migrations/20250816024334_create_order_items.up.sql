-- Add up migration script here
CREATE TABLE "order_items" (
    "order_item_id" SERIAL PRIMARY KEY,
    "order_id" INT NOT NULL,
    "product_id" INT NOT NULL,
    "quantity" INT NOT NULL,
    "price" INT NOT NULL,
    "created_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    "deleted_at" TIMESTAMP DEFAULT NULL
);

CREATE INDEX "idx_order_items_order_id" ON "order_items" ("order_id");

CREATE INDEX "idx_order_items_product_id" ON "order_items" ("product_id");