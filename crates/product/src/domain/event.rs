use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderItemEvent {
    pub product_id: i32,
    pub quantity: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderItemUpdateEvent {
    pub product_id: i32,
    pub old_quantity: i32,
    pub new_quantity: i32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum OrderEvent {
    Created {
        order_id: i32,
        user_id: i32,
        items: Vec<OrderItemEvent>,
    },
    Updated {
        order_id: i32,
        updates: Vec<OrderItemUpdateEvent>,
    },
    Deleted {
        order_id: i32,
        deleted_items: Vec<OrderItemEvent>,
    },
}
