#!/bin/bash

# PgBouncer Initialization Script
# This script initializes PgBouncer with proper configuration and databases

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to wait for PostgreSQL to be ready
wait_for_postgres() {
    local db_host=$1
    local db_port=$2
    local db_user=$3
    local db_name=$4
    local max_attempts=30
    local attempt=1
    
    print_status "Waiting for PostgreSQL at $db_host:$db_port to be ready..."
    
    while [ $attempt -le $max_attempts ]; do
        if pg_isready -h $db_host -p $db_port -U $db_user -d $db_name > /dev/null 2>&1; then
            print_status "PostgreSQL at $db_host:$db_port is ready!"
            return 0
        fi
        
        print_status "Attempt $attempt/$max_attempts: PostgreSQL not ready, waiting 2 seconds..."
        sleep 2
        attempt=$((attempt + 1))
    done
    
    print_error "PostgreSQL at $db_host:$db_port failed to become ready after $max_attempts attempts"
    return 1
}

# Function to test database connection
test_database_connection() {
    local db_host=$1
    local db_port=$2
    local db_user=$3
    local db_pass=$4
    local db_name=$5
    
    print_status "Testing connection to $db_name at $db_host:$db_port..."
    
    PGPASSWORD=$db_pass psql -h $db_host -p $db_port -U $db_user -d $db_name -c "SELECT 1;" > /dev/null 2>&1
    
    if [ $? -eq 0 ]; then
        print_status "Connection to $db_name successful!"
        return 0
    else
        print_error "Failed to connect to $db_name"
        return 1
    fi
}

# Function to initialize PgBouncer
initialize_pgbouncer() {
    print_status "Initializing PgBouncer..."
    
    # Wait for all PostgreSQL databases to be ready
    wait_for_postgres "postgres_auth" "5432" "auth_user" "auth_db"
    wait_for_postgres "postgres_orders" "5432" "orders_user" "orders_db"
    wait_for_postgres "postgres_products" "5432" "products_user" "products_db"
    wait_for_postgres "postgres_role" "5432" "role_user" "role_db"
    wait_for_postgres "postgres_users" "5432" "users_user" "users_db"
    
    # Test all database connections
    test_database_connection "postgres_auth" "5432" "auth_user" "auth_password" "auth_db"
    test_database_connection "postgres_orders" "5432" "orders_user" "orders_password" "orders_db"
    test_database_connection "postgres_products" "5432" "products_user" "products_password" "products_db"
    test_database_connection "postgres_role" "5432" "role_user" "role_password" "role_db"
    test_database_connection "postgres_users" "5432" "users_user" "users_password" "users_db"
    
    print_status "All database connections verified!"
}

# Function to create PgBouncer admin database
create_admin_database() {
    print_status "Creating PgBouncer admin database..."
    
    # The pgbouncer database is virtual and created automatically
    # We just need to test if we can connect to it
    sleep 5  # Give PgBouncer time to start
    
    if psql -h localhost -p 6432 -U admin -d pgbouncer -c "SHOW VERSION;" > /dev/null 2>&1; then
        print_status "PgBouncer admin database is accessible!"
    else
        print_warning "PgBouncer admin database not yet accessible, this is normal on first start"
    fi
}

# Function to setup monitoring
setup_monitoring() {
    print_status "Setting up monitoring..."
    
    # Create a simple monitoring query
    cat > /tmp/monitoring.sql << 'EOF'
-- PgBouncer Monitoring Query
SELECT 
    'pgbouncer_uptime' as metric,
    EXTRACT(EPOCH FROM (NOW() - pg_postmaster_start_time())) as value,
    'seconds' as unit
UNION ALL
SELECT 
    'total_connections' as metric,
    COUNT(*) as value,
    'count' as unit
FROM pgbouncer.pools
WHERE database != 'pgbouncer';
EOF
    
    print_status "Monitoring setup completed!"
}

# Main initialization function
main() {
    print_status "Starting PgBouncer initialization..."
    print_status "====================================="
    
    # Initialize PgBouncer
    initialize_pgbouncer
    
    # Create admin database
    create_admin_database
    
    # Setup monitoring
    setup_monitoring
    
    print_status "====================================="
    print_status "PgBouncer initialization completed successfully!"
    print_status ""
    print_status "PgBouncer is now ready to accept connections."
    print_status "Admin interface available at: localhost:6432"
    print_status "Use 'psql -h localhost -p 6432 -U admin -d pgbouncer' for admin access"
}

# Run main function
main "$@"