# component-fast2flow

Intelligent chat-to-flow routing components for Greentic.

## Overview

This workspace provides components for fast, deterministic intent matching with optional LLM fallback:

- **fast2flow.indexer**: Build search index from flow metadata on pack deploy
- **fast2flow.matcher**: Fast BM25-based matching (<100ms target latency)
- **fast2flow.router**: Full routing logic with confidence thresholds and LLM fallback

## Architecture

```
request -> control-chain -> message post-hook -> fast2flow ->
  if <100ms reply: return deterministic match
  else: call LLM -> continue/respond/forward/deny
```

## Control Directive Actions

| Action | Description |
|--------|-------------|
| `Continue` | Pass through, no routing decision |
| `Dispatch { target }` | Route to specific pack/flow |
| `Respond { reply }` | Ask clarifying question ("Did you mean...?") |
| `Deny { reply }` | Reject query (not acceptable) |

## Components

### fast2flow.indexer

Extracts flow metadata from packs and builds a searchable index:
- Flow IDs, titles, descriptions
- Tags and keywords
- Parameter definitions

Index is stored in state store (Redis/Memory) for fast retrieval.

### fast2flow.matcher

Implements BM25 (Best Matching 25) algorithm for fast text matching:
- Sub-100ms latency target
- Configurable confidence threshold (default: 0.7)
- Returns top candidates with confidence scores

### fast2flow.router

Orchestrates the full routing pipeline:
1. Fast match against index
2. If high confidence: dispatch immediately
3. If ambiguous: ask clarifying question
4. If low confidence: call LLM for semantic understanding
5. If blocked: deny with reason

## Index Structure

```json
{
  "version": "1.0",
  "last_updated": "2026-03-11T10:00:00Z",
  "flows": [
    {
      "pack_id": "customer-support",
      "flow_id": "book_appointment",
      "title": "Book Appointment",
      "description": "Schedule appointments with calendar",
      "tags": ["booking", "calendar", "appointment"],
      "keywords": ["book", "schedule", "meeting"]
    }
  ]
}
```

## Hook Integration

Integrates via post-ingress hooks:

```yaml
extensions:
  greentic.ext.offers.v1:
    inline:
      offers:
        - id: fast2flow-hook
          kind: hook
          stage: post_ingress
          contract: greentic.hook.control.v1
          priority: 50
          provider:
            op: route
```

## Building

```bash
make build   # Build all components
make wasm    # Build WASM components
make test    # Run tests
```

## License

MIT
