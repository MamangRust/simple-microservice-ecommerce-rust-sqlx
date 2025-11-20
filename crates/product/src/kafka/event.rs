use crate::{
    abstract_trait::product::service::DynProductCommandService,
    domain::event::{OrderEvent, OrderItemEvent},
};
use shared::errors::ServiceError;
use tracing::{error, info};

pub struct OrderEventHandler {
    pub product_service: DynProductCommandService,
}

impl OrderEventHandler {
    pub fn new(product_service: DynProductCommandService) -> Self {
        Self { product_service }
    }

    pub async fn handle_event(&self, event: OrderEvent) -> Result<(), ServiceError> {
        match event {
            OrderEvent::Created {
                order_id, items, ..
            } => {
                info!("📦 Processing Created event for order_id={}", order_id);
                for OrderItemEvent {
                    product_id,
                    quantity,
                } in items
                {
                    info!(
                        "🔁 Reducing stock for product_id={} by {} (Created event)",
                        product_id, quantity
                    );

                    if let Err(e) = self
                        .product_service
                        .decreasing_stock(product_id, quantity)
                        .await
                    {
                        error!(
                            "❌ Failed to decrease stock for product_id={}: {}",
                            product_id, e
                        );
                        return Err(e);
                    }
                }
                Ok(())
            }
            OrderEvent::Updated {
                order_id, updates, ..
            } => {
                info!("🔄 Processing Updated event for order_id={}", order_id);
                for update in updates {
                    let product_id = update.product_id;
                    let old_quantity = update.old_quantity;
                    let new_quantity = update.new_quantity;

                    let delta = new_quantity as i64 - old_quantity as i64;

                    if delta > 0 {
                        info!(
                            "🔁 Reducing stock for product_id={} by {} (old: {}, new: {})",
                            product_id, delta, old_quantity, new_quantity
                        );
                        if let Err(e) = self
                            .product_service
                            .decreasing_stock(product_id, delta as i32)
                            .await
                        {
                            error!(
                                "❌ Failed to decrease stock for product_id={}: {}",
                                product_id, e
                            );
                            return Err(e);
                        }
                    } else if delta < 0 {
                        let abs_delta = (-delta) as i32;
                        info!(
                            "🔁 Increasing stock for product_id={} by {} (old: {}, new: {})",
                            product_id, abs_delta, old_quantity, new_quantity
                        );
                        if let Err(e) = self
                            .product_service
                            .increasing_stock(product_id, abs_delta)
                            .await
                        {
                            error!(
                                "❌ Failed to increase stock for product_id={}: {}",
                                product_id, e
                            );
                            return Err(e);
                        }
                    } else {
                        info!(
                            "ℹ️ No stock change needed for product_id={} (quantity unchanged: {})",
                            product_id, new_quantity
                        );
                    }
                }
                Ok(())
            }
            OrderEvent::Deleted {
                order_id,
                deleted_items,
                ..
            } => {
                info!("🗑️ Processing Deleted event for order_id={}", order_id);
                for OrderItemEvent {
                    product_id,
                    quantity,
                } in deleted_items
                {
                    info!(
                        "🔁 Restoring stock for product_id={} by {} (Deleted event)",
                        product_id, quantity
                    );
                    if let Err(e) = self
                        .product_service
                        .increasing_stock(product_id, quantity)
                        .await
                    {
                        error!(
                            "❌ Failed to increase stock for product_id={}: {}",
                            product_id, e
                        );
                        return Err(e);
                    }
                }
                Ok(())
            }
        }
    }
}
