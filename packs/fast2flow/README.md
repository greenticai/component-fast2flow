# fast2flow Pack

Intelligent chat-to-flow routing with fast deterministic intent matching.

## Overview

fast2flow provides automatic routing of user messages to the appropriate flow based on intent detection:

1. **Indexer**: Builds a searchable index from flow metadata on pack deploy
2. **Matcher**: Fast BM25-based matching (<100ms target latency)
3. **Router**: Makes routing decisions with optional LLM fallback

## Architecture

```
request -> control-chain -> message post-hook -> fast2flow ->
  if <100ms reply: return deterministic match
  else: call LLM -> continue/respond/forward/deny
```

## Control Directive Actions

| Action | Description |
|--------|-------------|
| `continue` | Pass through, no routing decision |
| `dispatch` | Route to specific pack/flow |
| `respond` | Ask clarifying question ("Did you mean...?") |
| `deny` | Reject query (not acceptable) |

## Installation

```bash
greentic-pack install fast2flow.gtpack
```

## Configuration

### Index Build

The index is automatically built when packs are deployed. You can also trigger manual rebuilds:

```yaml
flows:
  - id: my_flow
    title: "Book Appointment"
    description: "Schedule appointments with calendar"
    tags:
      - booking
      - calendar
    keywords:
      - book
      - schedule
      - meeting
```

### Router Configuration

```yaml
parameters:
  confidence_threshold: 0.7    # Minimum confidence for dispatch
  enable_llm_fallback: false   # Enable LLM for low-confidence matches
  blocked_intents:             # Intents to always deny
    - admin:delete_all
    - dangerous_pack
```

## Flow Examples

### Basic Routing

```yaml
nodes:
  route:
    fast2flow.router:
      op: route
      message: "{{input}}"
      match_result: "{{match}}"
      tenant_id: "{{context.tenant_id}}"
      config:
        confidence_threshold: 0.8
```

### With Hook Integration

Configure as post-ingress hook in pack.yaml:

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

## BM25 Algorithm

fast2flow uses BM25 (Best Matching 25) for fast, deterministic text matching:

- **K1 parameter**: 1.5 (term frequency saturation)
- **B parameter**: 0.75 (document length normalization)
- **Tokenization**: Lowercase, split on whitespace/punctuation
- **Minimum term length**: 2 characters

## Performance

Target metrics:
- Matching latency: <100ms for 95th percentile
- Index size: O(flows * avg_terms)
- Memory: Depends on index size, typically <10MB for 1000 flows

## License

MIT
