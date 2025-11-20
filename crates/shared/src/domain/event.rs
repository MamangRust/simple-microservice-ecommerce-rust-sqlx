use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum OrderEvent {
    Created {
        order_id: i32,
        product_id: i32,
        quantity: i32,
    },
    Updated {
        order_id: i32,
        product_id: i32,
        old_quantity: i32,
        new_quantity: i32,
    },
    Deleted {
        order_id: i32,
        product_id: i32,
        quantity: i32,
    },
}
