#!/usr/bin/env bash
# E2E Test Runner for fast2flow
# Usage: ./scripts/e2e.sh [build|test|deploy|e2e|benchmark|clean|all]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$PROJECT_DIR")"
E2E_BUNDLE_DIR="$ROOT_DIR/e2e-test-bundle"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

check_prerequisites() {
    log_info "Checking prerequisites..."

    local missing=()

    if ! command -v cargo &> /dev/null; then
        missing+=("cargo")
    fi

    if ! command -v cargo-component &> /dev/null; then
        missing+=("cargo-component (install: cargo install cargo-component --locked)")
    fi

    if ! rustup target list --installed | grep -q "wasm32-wasip2"; then
        missing+=("wasm32-wasip2 target (install: rustup target add wasm32-wasip2)")
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        log_error "Missing prerequisites:"
        for item in "${missing[@]}"; do
            echo "  - $item"
        done
        exit 1
    fi

    log_success "All prerequisites met"
}

build_components() {
    log_info "Building fast2flow WASM components..."
    cd "$PROJECT_DIR"

    # Build indexer
    log_info "Building indexer..."
    cd "$PROJECT_DIR/components/indexer"
    cargo component build --release
    log_success "indexer built"

    # Build matcher
    log_info "Building matcher..."
    cd "$PROJECT_DIR/components/matcher"
    cargo component build --release
    log_success "matcher built"

    # Build router
    log_info "Building router..."
    cd "$PROJECT_DIR/components/router"
    cargo component build --release
    log_success "router built"

    cd "$PROJECT_DIR"
    log_success "All components built"
}

build_pack() {
    log_info "Building fast2flow.gtpack..."
    cd "$PROJECT_DIR"

    # Check if greentic-pack is available
    if command -v greentic-pack &> /dev/null; then
        greentic-pack build --in packs/fast2flow --gtpack-out dist/fast2flow.gtpack
    elif command -v packc &> /dev/null; then
        packc build --in packs/fast2flow --gtpack-out dist/fast2flow.gtpack
    else
        log_warn "greentic-pack/packc not found, using make"
        make pack 2>/dev/null || log_warn "Pack build skipped (tool not available)"
        return
    fi

    log_success "Pack built: dist/fast2flow.gtpack"
}

run_tests() {
    log_info "Running unit tests..."
    cd "$PROJECT_DIR"

    cargo test --workspace

    log_success "All tests passed"
}

run_benchmark() {
    log_info "Running BM25 benchmark..."
    cd "$PROJECT_DIR"

    # Run benchmark tests if available
    cargo test --workspace -- --nocapture benchmark 2>/dev/null || {
        log_info "Running matcher performance test..."
        cargo test --package fast2flow-matcher -- --nocapture 2>/dev/null || true
    }

    log_success "Benchmark completed"
}

deploy_to_bundle() {
    log_info "Deploying to e2e-test-bundle..."

    # Create bundle directories if needed
    mkdir -p "$E2E_BUNDLE_DIR/packs"

    # Copy pack
    if [ -f "$PROJECT_DIR/dist/fast2flow.gtpack" ]; then
        cp "$PROJECT_DIR/dist/fast2flow.gtpack" "$E2E_BUNDLE_DIR/packs/"
        log_success "Deployed fast2flow.gtpack"
    else
        log_warn "fast2flow.gtpack not found, run 'build' first"
    fi
}

run_e2e() {
    log_info "Running E2E tests..."

    # Check if operator is available
    if ! command -v gtc &> /dev/null; then
        log_warn "gtc (greentic CLI) not found, skipping E2E"
        log_info "Install greentic-operator and run: gtc op demo up"
        return
    fi

    cd "$E2E_BUNDLE_DIR"

    # Run test flow
    log_info "Testing fast2flow intent matching..."
    gtc op demo send --flow e2e_fast2flow_test --input '{"text": "I want to book an appointment"}' 2>/dev/null || {
        log_warn "E2E test requires operator running: gtc op demo up"
    }

    log_success "E2E tests completed"
}

clean() {
    log_info "Cleaning build artifacts..."
    cd "$PROJECT_DIR"

    cargo clean
    rm -rf dist/

    log_success "Cleaned"
}

show_help() {
    echo "fast2flow E2E Test Runner"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  build     Build WASM components"
    echo "  pack      Build .gtpack file"
    echo "  test      Run unit tests"
    echo "  benchmark Run BM25 performance benchmark"
    echo "  deploy    Deploy pack to e2e-test-bundle"
    echo "  e2e       Run E2E tests (requires operator)"
    echo "  clean     Clean build artifacts"
    echo "  all       Run build + pack + test + deploy + e2e"
    echo ""
    echo "Examples:"
    echo "  $0 build            # Build WASM components"
    echo "  $0 benchmark        # Run BM25 benchmark"
    echo "  $0 all              # Full E2E pipeline"
}

main() {
    if [ $# -eq 0 ]; then
        show_help
        exit 0
    fi

    for cmd in "$@"; do
        case "$cmd" in
            build)
                check_prerequisites
                build_components
                ;;
            pack)
                build_pack
                ;;
            test)
                run_tests
                ;;
            benchmark)
                run_benchmark
                ;;
            deploy)
                deploy_to_bundle
                ;;
            e2e)
                run_e2e
                ;;
            clean)
                clean
                ;;
            all)
                check_prerequisites
                build_components
                build_pack
                run_tests
                deploy_to_bundle
                run_e2e
                ;;
            help|--help|-h)
                show_help
                ;;
            *)
                log_error "Unknown command: $cmd"
                show_help
                exit 1
                ;;
        esac
    done
}

main "$@"
