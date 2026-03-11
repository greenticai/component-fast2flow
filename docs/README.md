# fast2flow - Intelligent Chat-to-Flow Routing

Fast deterministic intent matching with BM25 algorithm for automatic flow routing.

## Overview

fast2flow provides intelligent routing of user messages to appropriate flows based on intent detection:

| Component | Purpose | Target Latency |
|-----------|---------|----------------|
| `indexer` | Builds searchable index from flow metadata | On deploy |
| `matcher` | BM25-based intent matching | <100ms |
| `router` | Makes routing decisions with optional LLM fallback | <100ms (fast path) |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     fast2flow Pipeline                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │   Indexer    │───▶│    Matcher   │───▶│    Router    │  │
│  │  (on deploy) │    │   (BM25)     │    │  (decision)  │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│         │                   │                   │           │
│         ▼                   ▼                   ▼           │
│  ┌────────────────────────────────────────────────────────┐ │
│  │              State Store (Redis/Memory)                 │ │
│  │  key: fast2flow:index:{tenant}:{team}                  │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ControlDirective Output
                    (continue|dispatch|respond|deny)
```

## Installation

### Build from Source

```bash
# Prerequisites
cargo install cargo-component --locked

# Build WASM components
make wasm

# Build pack
make pack

# Output: dist/fast2flow.gtpack
```

### Deploy Pack

```bash
# Copy to demo bundle
cp dist/fast2flow.gtpack ~/my-bundle/packs/

# Or install via operator
gtc op pack install dist/fast2flow.gtpack
```

## Usage

### Step 1: Build Flow Index

Run on pack deploy to index all flow metadata:

```yaml
id: build_index
type: events
start: build

nodes:
  build:
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
          keywords: ["order", "status", "track", "where"]
      tenant_id: "{{context.tenant_id}}"
      team_id: "{{context.team_id}}"
    routing:
      - to: store

  store:
    state.set:
      key: "{{build.index_key}}"
      value: "{{build.index}}"
    routing:
      - out: true
```

### Step 2: Match User Intent

```yaml
id: match_intent
type: messaging
start: load_index

nodes:
  load_index:
    state.get:
      key: "fast2flow:index:{{context.tenant_id}}:{{context.team_id}}"
    routing:
      - to: match

  match:
    fast2flow.matcher:
      op: match
      query: "{{input.text}}"
      index: "{{load_index.value}}"
      threshold: 0.7
      max_results: 5
    routing:
      - out: true
```

### Step 3: Route Message

```yaml
id: route_message
type: messaging
start: match

nodes:
  match:
    flow.call:
      flow: match_intent
      input:
        text: "{{input.text}}"
    routing:
      - to: route

  route:
    fast2flow.router:
      op: route
      message:
        id: "{{input.id}}"
        text: "{{input.text}}"
        channel: "{{input.channel}}"
        session_id: "{{input.session_id}}"
      match_result: "{{match}}"
      tenant_id: "{{context.tenant_id}}"
      config:
        confidence_threshold: 0.8
        enable_llm_fallback: false
        blocked_intents:
          - "admin:delete_all"
    routing:
      - out: true
```

## Control Directive Actions

| Action | Description | When Used |
|--------|-------------|-----------|
| `continue` | Pass through, no routing decision | No match found, let next handler process |
| `dispatch` | Route to specific pack/flow | High-confidence match |
| `respond` | Ask clarifying question | Ambiguous match (multiple candidates) |
| `deny` | Reject query | Blocked intent or policy violation |

### Dispatch Output

```json
{
  "action": "dispatch",
  "target": {
    "tenant": "acme",
    "pack": "customer-support",
    "flow": "book_appointment"
  }
}
```

### Respond Output

```json
{
  "action": "respond",
  "response_text": "Did you mean:\n1. Book Appointment\n2. Check Order Status"
}
```

### Deny Output

```json
{
  "action": "deny",
  "reason_code": "blocked_intent",
  "response_text": "This action is not permitted."
}
```

## BM25 Algorithm

fast2flow uses BM25 (Best Matching 25) for fast, deterministic text matching:

### Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| K1 | 1.5 | Term frequency saturation |
| B | 0.75 | Document length normalization |
| Min Term Length | 2 | Minimum characters per token |

### Scoring

```
score(D, Q) = Σ IDF(qi) * (tf(qi, D) * (k1 + 1)) / (tf(qi, D) + k1 * (1 - b + b * |D|/avgdl))
```

Where:
- `tf(qi, D)` = term frequency of qi in document D
- `IDF(qi)` = inverse document frequency
- `|D|` = document length
- `avgdl` = average document length

### Tokenization

1. Convert to lowercase
2. Split on whitespace and punctuation
3. Filter tokens < 2 characters
4. Optional: stem words (not implemented in v0.1)

## Performance

### Target Metrics

| Metric | Target | Notes |
|--------|--------|-------|
| Match latency (p50) | <50ms | Fast path |
| Match latency (p95) | <100ms | Including edge cases |
| Match latency (p99) | <200ms | With complex queries |
| Index size | O(flows × terms) | Memory efficient |
| Memory usage | <10MB | For 1000 flows |

### Optimization Tips

1. **Keep descriptions concise** - 50-100 words max
2. **Use specific keywords** - Avoid generic terms
3. **Limit tags** - 3-5 tags per flow
4. **Pre-filter queries** - Remove stopwords client-side

## Configuration

### Index Structure

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
      "tags": ["booking", "calendar"],
      "keywords": ["book", "schedule", "meeting"]
    }
  ],
  "term_frequencies": { ... },
  "document_frequencies": { ... }
}
```

### Router Configuration

```yaml
config:
  confidence_threshold: 0.7    # Min confidence for dispatch
  ambiguity_threshold: 0.1     # Max diff between top matches
  enable_llm_fallback: false   # Use LLM for low-confidence
  llm_prompt_template: "..."   # Custom LLM prompt
  blocked_intents:             # Always deny these
    - "admin:*"
    - "dangerous_pack"
```

## Hook Integration

Use as post-ingress hook for automatic routing:

```yaml
# pack.yaml
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

## Testing

```bash
# Unit tests
cargo test --workspace

# BM25 benchmark
../scripts/e2e-fast2flow.sh benchmark

# E2E tests
../scripts/e2e-fast2flow.sh all
```

## Examples

### Customer Support Bot

```yaml
id: support_router
type: messaging
start: route

parameters:
  confidence_threshold: 0.75

nodes:
  route:
    fast2flow.router:
      op: route
      message: "{{input}}"
      match_result: "{{context.match_result}}"
      config:
        confidence_threshold: "{{parameters.confidence_threshold}}"
    routing:
      - to: dispatch
        when: "{{route.action == 'dispatch'}}"
      - to: clarify
        when: "{{route.action == 'respond'}}"
      - to: fallback
        when: "{{route.action == 'continue'}}"

  dispatch:
    flow.call:
      pack: "{{route.target.pack}}"
      flow: "{{route.target.flow}}"
      input: "{{input}}"
    routing:
      - out: true

  clarify:
    emit.response:
      text: "{{route.response_text}}"
    routing:
      - out: true

  fallback:
    flow.call:
      flow: "default_handler"
    routing:
      - out: true
```

## API Reference

See [API.md](./API.md) for complete API documentation.

## License

MIT (Commercial feature)
