# Simple E-Commerce Microservices

This project is a simple e-commerce application built with a microservices architecture using Rust.

## Architecture Overview

The architecture consists of several microservices that communicate with each other via gRPC. An API Gateway serves as the single entry point for all client requests and routes them to the appropriate service.

- **API Gateway**: The single entry point for all client requests.
- **Services**: Each service is responsible for a specific business domain.
- **gRPC**: Used for communication between services.
- **Database**: Each service has its own database (PostgreSQL).

## Services

- **API Gateway**: Handles and routes incoming requests to the appropriate microservice.
- **Auth Service**: Manages user authentication and authorization.
- **User Service**: Manages user data.
- **Role Service**: Manages user roles and permissions.
- **Product Service**: Manages product information.
- **Order Service**: Manages customer orders.
- **Email Service**: Handles sending emails.

## Technologies Used

- [Rust](https://www.rust-lang.org/)
- [Tonic](https://github.com/hyperium/tonic) for gRPC
- [SQLx](https://github.com/launchbadge/sqlx) for database interaction
- [PostgreSQL](https://www.postgresql.org/)
- [Docker](https://www.docker.com/)

## How to Run

1.  **Prerequisites**:

    - [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/) installed.
    - [Rust](https://www.rust-lang.org/tools/install) toolchain.

2.  **Clone the repository**:

    ```bash
    git clone https://github.com/MamangRust/microservice-simple-ecommerce.git
    cd microservice-simple-ecommerce
    ```

3.  **Run the application**:

    ```bash
    make up
    ```

    This command will start all the services using Docker Compose.

4.  **Stop the application**:
    ```bash
    make down
    ```

## Available Commands

- `make up`: Start all services in detached mode.
- `make down`: Stop and remove all services.
- `make build-genproto`: Build the `genproto` crate.
- `make build-<service-name>`: Build a specific service (e.g., `make build-apigateway`).
- `make clipy`: Run clippy for all crates.
- `make fmt`: Check formatting for all crates.
