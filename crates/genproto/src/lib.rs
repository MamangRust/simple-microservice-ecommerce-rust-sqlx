pub mod api {
    include!("gen/api.rs");
}

pub mod common {
    include!("gen/common.rs");
}

pub mod auth {
    include!("gen/auth.rs");
}

pub mod user {
    include!("gen/user.rs");
}

pub mod role {
    include!("gen/role.rs");
}

pub mod product {
    include!("gen/product.rs");
}

pub mod order {
    include!("gen/order.rs");
}

pub mod order_item {
    include!("gen/order_item.rs");
}

pub mod user_role {
    include!("gen/user_role.rs");
}
