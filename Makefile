build-genproto:
	cargo build -p genproto

build-apigateway:
	cargo build -p apigateway

build-auth:
	cargo build -p auth

build-email:
	cargo build -p email

build-product:
	cargo build -p product

build-order:
	cargo build -p order

clipy:
	SQLX_OFFLINE=true cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all -- --check

up:
	docker compose up -d

down:
	docker compose down
