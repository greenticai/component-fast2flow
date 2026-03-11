#!/usr/bin/env bash
# Setup E2E Test Bundle for fast2flow
# Creates the necessary directory structure and test files

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$PROJECT_DIR")"
E2E_BUNDLE_DIR="$ROOT_DIR/e2e-test-bundle"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }

setup_directories() {
    log_info "Creating directory structure..."
    mkdir -p "$E2E_BUNDLE_DIR"/{packs,flows,tenants/test/teams/default,state,logs}
    log_success "Directories created"
}

create_test_flow() {
    log_info "Creating fast2flow test flow..."

    cat > "$E2E_BUNDLE_DIR/flows/e2e_fast2flow_test.ygtc" << 'EOF'
id: e2e_fast2flow_test
title: E2E Test - Fast Intent Routing
description: End-to-end test for fast2flow intent matching and routing
type: messaging
start: index_flows

parameters:
  tenant_id: "test"
  team_id: "default"

nodes:
  index_flows:
    fast2flow.indexer:
      op: build
      flows:
        - pack_id: "customer-support"
          flow_id: "book_appointment"
          title: "Book Appointment"
          description: "Schedule appointments with calendar"
          tags: ["booking", "calendar"]
          keywords: ["book", "schedule", "meeting", "appointment"]
        - pack_id: "customer-support"
          flow_id: "check_order"
          title: "Check Order Status"
          description: "Track and check order status"
          tags: ["order", "tracking"]
          keywords: ["order", "status", "track", "delivery"]
        - pack_id: "customer-support"
          flow_id: "cancel_subscription"
          title: "Cancel Subscription"
          description: "Cancel active subscription"
          tags: ["subscription", "cancel"]
          keywords: ["cancel", "subscription", "stop", "end"]
      tenant_id: "{{parameters.tenant_id}}"
      team_id: "{{parameters.team_id}}"
    routing:
      - to: match

  match:
    fast2flow.matcher:
      op: match
      query: "{{input.text | default: 'I want to book an appointment'}}"
      index: "{{index_flows.index}}"
      threshold: 0.5
      max_results: 3
    routing:
      - to: route

  route:
    fast2flow.router:
      op: route
      message:
        id: "msg-e2e-001"
        text: "{{input.text}}"
        channel: "e2e-test"
        session_id: "e2e-session"
      match_result: "{{match}}"
      tenant_id: "{{parameters.tenant_id}}"
      team_id: "{{parameters.team_id}}"
      config:
        confidence_threshold: 0.7
        enable_llm_fallback: false
    routing:
      - to: verify

  verify:
    templating.handlebars:
      text: |
        {
          "status": "success",
          "action": "{{route.action}}",
          "target": {{json_encode route.target}},
          "match_status": "{{match.status}}",
          "top_match": {{json_encode match.top_match}},
          "latency_ms": {{match.latency_ms}}
        }
    routing:
      - out: true
EOF

    log_success "Test flow created: flows/e2e_fast2flow_test.ygtc"
}

create_tenant_config() {
    log_info "Creating tenant configuration..."

    mkdir -p "$E2E_BUNDLE_DIR/tenants/test/teams/default"

    cat > "$E2E_BUNDLE_DIR/tenants/test/tenant.gmap" << 'EOF'
{
  "tenant_id": "test",
  "display_name": "E2E Test Tenant",
  "environment": "dev",
  "features": {
    "fast2flow": true
  }
}
EOF

    cat > "$E2E_BUNDLE_DIR/tenants/test/teams/default/team.gmap" << 'EOF'
{
  "team_id": "default",
  "display_name": "Default Team"
}
EOF

    log_success "Tenant configuration created"
}

create_demo_config() {
    log_info "Creating greentic.demo.yaml..."

    if [ ! -f "$E2E_BUNDLE_DIR/greentic.demo.yaml" ]; then
        cat > "$E2E_BUNDLE_DIR/greentic.demo.yaml" << 'EOF'
version: "1"
project_root: "./"
tenant: "test"
team: "default"
environment: "dev"

services:
  nats:
    enabled: true
    spawn:
      enabled: true
      port: 4222

logging:
  level: "info"
  format: "pretty"

http:
  host: "127.0.0.1"
  port: 8080

packs:
  fast2flow:
    path: "packs/fast2flow.gtpack"
    enabled: true
EOF
        log_success "greentic.demo.yaml created"
    else
        log_info "greentic.demo.yaml already exists, skipping"
    fi
}

main() {
    log_info "Setting up E2E Test Bundle for fast2flow..."

    setup_directories
    create_test_flow
    create_tenant_config
    create_demo_config

    echo ""
    log_success "E2E Test Bundle setup complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Build:     ./scripts/e2e.sh build"
    echo "  2. Pack:      ./scripts/e2e.sh pack"
    echo "  3. Deploy:    ./scripts/e2e.sh deploy"
    echo "  4. Benchmark: ./scripts/e2e.sh benchmark"
    echo "  5. Run:       ./scripts/e2e.sh e2e"
    echo ""
}

main
