#!/bin/bash

echo "📊 MagicTunnel OAuth 2.1 Monitoring Dashboard"
echo "============================================="

# Function to get metrics
get_metrics() {
    curl -s http://localhost:3001/metrics | grep -E "auth_|session_|oauth_" || echo "No auth metrics available"
}

# Function to get health status
get_health() {
    curl -s http://localhost:3001/health | jq '.' 2>/dev/null || echo "Health endpoint not available"
}

# Function to get session status
get_sessions() {
    curl -s http://localhost:3001/admin/sessions/status | jq '.' 2>/dev/null || echo "Session status not available"
}

# Main monitoring loop
while true; do
    clear
    echo "📊 MagicTunnel OAuth 2.1 Monitoring Dashboard"
    echo "============================================="
    echo "📅 $(date)"
    echo ""
    
    echo "🏥 Health Status:"
    get_health
    echo ""
    
    echo "👥 Session Status:"
    get_sessions
    echo ""
    
    echo "📈 Authentication Metrics:"
    get_metrics
    echo ""
    
    echo "🔄 Refreshing in 10 seconds... (Ctrl+C to exit)"
    sleep 10
done
