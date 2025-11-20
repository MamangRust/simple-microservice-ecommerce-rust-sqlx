use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = "src/gen";

    fs::create_dir_all(out_dir)?;

    tonic_prost_build::configure()
        .build_server(true)
        .out_dir(out_dir)
        .compile_protos(
            &[
                "../../proto/api.proto",
                // user proto
                "../../proto/user/common.proto",
                "../../proto/user/command.proto",
                "../../proto/user/query.proto",
                // role proto
                "../../proto/role/common.proto",
                "../../proto/role/query.proto",
                "../../proto/role/command.proto",
                // product proto
                "../../proto/product/common.proto",
                "../../proto/product/query.proto",
                "../../proto/product/command.proto",
                // order proto
                "../../proto/order/common.proto",
                "../../proto/order/query.proto",
                "../../proto/order/command.proto",
                // auth proto
                "../../proto/auth/auth.proto",
                // user role proto
                "../../proto/user_role/user_role.proto",
                // order item proto
                "../../proto/order_item/orderitem.proto",
            ],
            &["../../proto"],
        )?;

    println!("cargo:rerun-if-changed=../../proto");

    Ok(())
}
