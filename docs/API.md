# fast2flow API Reference

## Components

### fast2flow.indexer

Builds and maintains the flow search index.

#### Operations

##### `build`

Build a complete index from flow metadata.

**Input:**
```typescript
interface BuildInput {
  flows: FlowEntry[];
  tenant_id: string;
  team_id?: string;
}

interface FlowEntry {
  pack_id: string;
  flow_id: string;
  title: string;
  description?: string;
  tags?: string[];
  keywords?: string[];
}
```

**Output:**
```typescript
interface BuildOutput {
  version: string;
  last_updated: string;
  flow_count: number;
  index_key: string;
  index: FlowIndex;
}
```

**Example:**
```yaml
fast2flow.indexer:
  op: build
  flows: "{{input.flows}}"
  tenant_id: "{{context.tenant_id}}"
  team_id: "{{context.team_id}}"
```

##### `update`

Incrementally update an existing index.

**Input:**
```typescript
interface UpdateInput {
  flows: FlowEntry[];
  tenant_id: string;
  team_id?: string;
  mode: "add" | "remove" | "replace";
}
```

**Example:**
```yaml
fast2flow.indexer:
  op: update
  flows: "{{input.new_flows}}"
  mode: "add"
  tenant_id: "{{context.tenant_id}}"
```

---

### fast2flow.matcher

Fast BM25-based intent matching.

#### Operations

##### `match`

Match a query against the flow index.

**Input:**
```typescript
interface MatchInput {
  query: string;
  index: FlowIndex;
  threshold?: number;   // default: 0.7
  max_results?: number; // default: 5
}
```

**Output:**
```typescript
interface MatchOutput {
  status: MatchStatus;
  top_match?: MatchResult;
  candidates: MatchResult[];
  latency_ms: number;
}

type MatchStatus = "match" | "ambiguous" | "no_match" | "timeout";

interface MatchResult {
  pack_id: string;
  flow_id: string;
  title: string;
  confidence: number; // 0.0 - 1.0
}
```

**Example:**
```yaml
fast2flow.matcher:
  op: match
  query: "{{input.text}}"
  index: "{{state.index}}"
  threshold: 0.7
  max_results: 5
```

**Match Status Meanings:**

| Status | Meaning |
|--------|---------|
| `match` | Single high-confidence match found |
| `ambiguous` | Multiple matches with similar scores |
| `no_match` | No matches above threshold |
| `timeout` | Matching exceeded time budget |

---

### fast2flow.router

Makes routing decisions based on match results.

#### Operations

##### `route`

Determine routing action based on match result.

**Input:**
```typescript
interface RouteInput {
  message: MessageInfo;
  match_result: MatchOutput;
  tenant_id: string;
  team_id?: string;
  config?: RouterConfig;
}

interface MessageInfo {
  id: string;
  text?: string;
  channel: string;
  session_id: string;
}

interface RouterConfig {
  confidence_threshold?: number;  // default: 0.7
  ambiguity_threshold?: number;   // default: 0.1
  enable_llm_fallback?: boolean;  // default: false
  llm_prompt_template?: string;
  blocked_intents?: string[];
}
```

**Output:**
```typescript
interface RouteOutput {
  action: ControlAction;
  target?: DispatchTarget;
  response_text?: string;
  response_card?: any;
  reason_code?: string;
  status_code?: number;
}

type ControlAction = "continue" | "dispatch" | "respond" | "deny";

interface DispatchTarget {
  tenant: string;
  team?: string;
  pack: string;
  flow?: string;
  node?: string;
}
```

**Example:**
```yaml
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
    blocked_intents:
      - "admin:delete"
```

---

## Schemas

### FlowIndex

Internal index structure (stored in state):

```typescript
interface FlowIndex {
  version: string;
  last_updated: string;
  flows: IndexedFlow[];
  term_frequencies: Record<string, Record<string, number>>;
  document_frequencies: Record<string, number>;
  average_doc_length: number;
}

interface IndexedFlow {
  pack_id: string;
  flow_id: string;
  title: string;
  description?: string;
  tags: string[];
  keywords: string[];
  doc_length: number;
}
```

### RouterConfig

Complete configuration options:

```typescript
interface RouterConfig {
  // Confidence threshold for direct dispatch
  confidence_threshold?: number;  // 0.0-1.0, default: 0.7

  // Max score difference to consider ambiguous
  ambiguity_threshold?: number;   // 0.0-1.0, default: 0.1

  // Enable LLM fallback for low confidence
  enable_llm_fallback?: boolean;  // default: false

  // Custom LLM prompt template
  llm_prompt_template?: string;

  // Intent patterns to always deny
  blocked_intents?: string[];     // e.g., ["admin:*", "dangerous"]
}
```

---

## Error Handling

### Indexer Errors

```json
{
  "error": "no flows provided",
  "code": "EMPTY_INPUT"
}
```

### Matcher Errors

```json
{
  "error": "index not found or invalid",
  "code": "INVALID_INDEX"
}
```

### Router Errors

```json
{
  "error": "blocked intent detected: admin:delete",
  "code": "BLOCKED_INTENT"
}
```

---

## Performance Tuning

### Index Optimization

```yaml
# Good: Specific keywords
keywords: ["book", "appointment", "schedule", "calendar"]

# Bad: Too generic
keywords: ["help", "please", "want", "need"]
```

### Query Preprocessing

Recommended preprocessing before matching:
1. Lowercase
2. Remove punctuation
3. Remove stopwords (optional)
4. Trim whitespace

### Threshold Tuning

| Use Case | Confidence | Ambiguity |
|----------|------------|-----------|
| High precision | 0.85 | 0.05 |
| Balanced | 0.70 | 0.10 |
| High recall | 0.50 | 0.20 |

---

## LLM Fallback (Optional)

When `enable_llm_fallback: true`, low-confidence matches are sent to LLM:

### Default Prompt

```
You are a routing assistant. Given a user message and candidate flows,
decide which flow to route to or if clarification is needed.

User message: "{{input.text}}"

Candidates:
{{#each candidates}}
- {{pack_id}}/{{flow_id}}: {{title}} - {{description}}
{{/each}}

Respond with JSON:
{
  "action": "dispatch" | "respond" | "deny" | "continue",
  "target": { "pack": "...", "flow": "..." },
  "response": "..."
}
```

### Custom Prompt

```yaml
config:
  enable_llm_fallback: true
  llm_prompt_template: |
    Analyze this customer query: "{{input.text}}"
    Available intents: {{candidates}}
    Return the best matching intent or ask for clarification.
```
