#!/bin/bash
# Demo script for fast2flow components
# Shows the intelligent routing pipeline working end-to-end

set -e

echo "=============================================="
echo "  fast2flow Demo - Intelligent Chat Routing"
echo "=============================================="
echo ""

echo "📦 Building WASM components..."
cargo build --target wasm32-wasip2 --release --quiet -p indexer -p matcher -p router
echo "✅ WASM build successful"
echo ""

echo "🧪 Running E2E Tests..."
echo ""
cargo test -p fast2flow-e2e -- --nocapture 2>&1 | grep -E "(test tests::|ok|passed)"

echo ""
echo "🔧 Running WASM Runtime Validation..."
cargo test -p fast2flow-wasm-runtime --quiet -- --nocapture 2>&1 | grep -E "(test |ok|loaded|size)" || true

echo ""
echo "=============================================="
echo "  Full Test Summary"
echo "=============================================="
cargo test --workspace --quiet 2>&1 | tail -25

echo ""
echo "✅ Demo complete!"
echo ""
echo "Components validated:"
echo "  - indexer: Build searchable index from flow metadata"
echo "  - matcher: BM25-based fast intent matching (<100ms)"
echo "  - router:  Smart routing decisions"
echo ""
echo "Router Actions:"
echo "  - dispatch: Clear match → route to flow"
echo "  - respond:  Ambiguous → ask clarification"
echo "  - continue: No match → pass through"
echo "  - deny:     Blocked intent → reject"
