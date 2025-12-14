#!/bin/bash

# PgBouncer Management Script
# This script provides utilities for managing PgBouncer

set -e

PGBOUNCER_HOST="localhost"
PGBOUNCER_PORT="6432"
PGBOUNCER_ADMIN_USER="admin"
PGBOUNCER_ADMIN_PASS="admin_password"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to execute PgBouncer admin command
execute_pgbouncer_cmd() {
    local cmd=$1
    echo "SHOW $cmd;" | psql -h $PGBOUNCER_HOST -p $PGBOUNCER_PORT -U $PGBOUNCER_ADMIN_USER -d pgbouncer
}

# Function to show PgBouncer status
show_status() {
    print_status "PgBouncer Status:"
    execute_pgbouncer_cmd "status"
    echo ""
    
    print_status "PgBouncer Pools:"
    execute_pgbouncer_cmd "pools"
    echo ""
    
    print_status "PgBouncer Servers:"
    execute_pgbouncer_cmd "servers"
    echo ""
    
    print_status "PgBouncer Clients:"
    execute_pgbouncer_cmd "clients"
    echo ""
}

# Function to show statistics
show_stats() {
    print_status "PgBouncer Statistics:"
    execute_pgbouncer_cmd "stats"
    echo ""
    
    print_status "PgBouncer Statistics Reset:"
    execute_pgbouncer_cmd "stats_reset"
    echo ""
}

# Function to reload configuration
reload_config() {
    print_status "Reloading PgBouncer configuration..."
    execute_pgbouncer_cmd "config"
    echo ""
    
    print_status "Reloading PgBouncer..."
    echo "RELOAD;" | psql -h $PGBOUNCER_HOST -p $PGBOUNCER_PORT -U $PGBOUNCER_ADMIN_USER -d pgbouncer
    print_status "Configuration reloaded successfully!"
}

# Function to pause/resume PgBouncer
pause_pgbouncer() {
    print_status "Pausing PgBouncer..."
    echo "PAUSE;" | psql -h $PGBOUNCER_HOST -p $PGBOUNCER_PORT -U $PGBOUNCER_ADMIN_USER -d pgbouncer
    print_status "PgBouncer paused!"
}

resume_pgbouncer() {
    print_status "Resuming PgBouncer..."
    echo "RESUME;" | psql -h $PGBOUNCER_HOST -p $PGBOUNCER_PORT -U $PGBOUNCER_ADMIN_USER -d pgbouncer
    print_status "PgBouncer resumed!"
}

# Function to show help
show_help() {
    echo "PgBouncer Management Script"
    echo ""
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  status      Show PgBouncer status, pools, servers, and clients"
    echo "  stats       Show PgBouncer statistics"
    echo "  reload      Reload PgBouncer configuration"
    echo "  pause       Pause PgBouncer (accepts no new connections)"
    echo "  resume      Resume PgBouncer (accepts new connections)"
    echo "  help        Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 status"
    echo "  $0 stats"
    echo "  $0 reload"
}

# Main script logic
case "${1:-help}" in
    "status")
        show_status
        ;;
    "stats")
        show_stats
        ;;
    "reload")
        reload_config
        ;;
    "pause")
        pause_pgbouncer
        ;;
    "resume")
        resume_pgbouncer
        ;;
    "help"|*)
        show_help
        ;;
esac