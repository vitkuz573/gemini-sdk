# Phase 7 Research — `PCck7e` Conversation Actions

## Source

Primary source: `.opencode/skills/spike-findings-gemini-sdk/sources/001-har-api-coverage/README.md`

## Captured Evidence

### HAR API Coverage (spike 001)

| RPC | Source-path | Inner payload (decoded) | Likely purpose |
|-----|-------------|-------------------------|----------------|
| `PCck7e` | `/app/<conversation>` | `["r_0958d664053635a6"]` | conversation action (e.g. regenerate/rating) |

### Spike 002 Confirmation

| RPC | Payload | Purpose | Status |
|-----|---------|---------|--------|
| `PCck7e` | `["r_0d35e86934785889"]` | Rating/feedback | 200 |

### Decoded `batchexecute` Entry

From `batchexecute_rpcids.json`:

```json
{
  "rpcid": "PCck7e",
  "source_path": "/app/5526eba489c959f5",
  "inner_shape": [
    "r_0958d664053635a6"
  ]
}
```

The source path contains the conversation id (`5526eba489c959f5`) and the inner
payload contains a single response id (`r_0958d664053635a6`).

## Inferred Payload Shapes

The frontend likely encodes the action type either by:

1. Wrapping the response id in a different top-level array per action:
   - Regenerate: `[["r_{response_id}"]]`
   - Rate: `[["r_{response_id}", {rating}]]`
   - Delete: `[["r_{response_id}"]]` with an additional marker
2. Adding a numeric opcode inside the inner array:
   - Regenerate: `["r_{response_id}", 1]`
   - Rate: `["r_{response_id}", 2, {rating}]`
   - Delete: `["r_{response_id}", 3]`

Because the HAR only contains one sample, the implementation starts with option 1
(the shape that directly matches the capture) and treats opcodes as a fallback
adjustment if fixture tests reveal it.

## Expected Response Shape

Batchexecute responses for `PCck7e` appear to follow the standard WIZ framing:

```text
)] } ' 

[["wrb.fr","PCck7e","[...]",null,null,null,"generic"]]
```

The inner payload string at index 2 is expected to contain a JSON value that
can be interpreted as a success/failure indicator. Common patterns in Gemini
responses:

- Empty array `[]` → accepted / no-op.
- `[1]` or `[true]` → success.
- Nested error object with `error` or `code` → failure.

The parser therefore treats any payload that can be decoded and does not contain
an explicit `error` field as success, while explicit errors map to failure.

## Open Questions

1. Does the frontend require a numeric opcode to distinguish regenerate/rate/delete?
2. Does a successful delete return a different shape than regenerate/rate?
3. Does the server return the new response id after regenerate, or does the caller
   need to refresh conversation state separately?

These questions are answered by the fixture tests in the plan. If the initial
shape does not match the captured behavior, the implementation must be adjusted
and this research document updated.
