#!/bin/sh

# Simple PgBouncer Health Check Script
# This script performs basic health checks on PgBouncer

set -e

PGBOUNCER_HOST="localhost"
PGBOUNCER_PORT="6432"

# Function to check if PgBouncer is responding
check_pgbouncer_connectivity() {
    echo "Checking PgBouncer connectivity..."

    # Use PGPASSWORD environment variable to avoid password prompt
    # Use SHOW command which is valid in pgbouncer admin interface
    if PGPASSWORD="admin_password" psql -h $PGBOUNCER_HOST -p $PGBOUNCER_PORT -U admin -d pgbouncer -c "SHOW HELP;" > /dev/null 2>&1; then
        echo "✓ PgBouncer is responding"
        return 0
    else
        echo "✗ PgBouncer is not responding"
        return 1
    fi
}

# Function to check if pgbouncer process is running
check_pgbouncer_process() {
    echo "Checking PgBouncer process..."

    if pgrep pgbouncer > /dev/null 2>&1; then
        echo "✓ PgBouncer process is running"
        return 0
    else
        echo "✗ PgBouncer process is not running"
        return 1
    fi
}

# Main health check function
main() {
    echo "PgBouncer Health Check"
    echo "======================"
    echo ""

    local overall_status=0

    # Check if process is running
    if ! check_pgbouncer_process; then
        overall_status=1
    fi

    echo ""

    # Check connectivity
    if ! check_pgbouncer_connectivity; then
        overall_status=1
    fi

    echo ""
    echo "Health check completed."

    if [ $overall_status -eq 0 ]; then
        echo "✓ Overall health: GOOD"
        exit 0
    else
        echo "✗ Overall health: ISSUES DETECTED"
        exit 1
    fi
}

# Run main function
main "$@"