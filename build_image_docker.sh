#!/bin/bash

# Build Docker images for all microservices
set -e

echo "Building Docker images for microservices..."

# Build API Gateway image
echo "Building API Gateway image..."
docker build -f Dockerfile.apigateway -t microservice-ecommerce/apigateway:latest .

# Build Auth service image
echo "Building Auth service image..."
docker build -f Dockerfile.auth -t microservice-ecommerce/auth:latest .

# Build Email service image
echo "Building Email service image..."
docker build -f Dockerfile.email -t microservice-ecommerce/email:latest .

# Build Order service image
echo "Building Order service image..."
docker build -f Dockerfile.order -t microservice-ecommerce/order:latest .

# Build Product service image
echo "Building Product service image..."
docker build -f Dockerfile.product -t microservice-ecommerce/product:latest .

# Build Role service image
echo "Building Role service image..."
docker build -f Dockerfile.role -t microservice-ecommerce/role:latest .

# Build User service image
echo "Building User service image..."
docker build -f Dockerfile.user -t microservice-ecommerce/user:latest .

echo "All Docker images built successfully!"
echo "Available images:"
docker images | grep microservice-ecommerce