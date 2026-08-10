# Phase 8 Research — `o30O0e` User Info & `L5adhe` Last Selected Mode

## Source

Primary source: `.opencode/skills/spike-findings-gemini-sdk/sources/001-har-api-coverage/README.md`
(also referred to as the spike 009 capture of the exact RPC shapes).

## Captured Evidence

### HAR API Coverage

| RPC | Source-path | Inner payload (decoded) | Likely purpose |
|-----|-------------|-------------------------|----------------|
| `o30O0e` | `/` | `[["me"], [[["person.photo","person.name","person.email"]], null, [1,7]]]` | user info |
| `L5adhe` | `/` | `[[null,...null, "cf41b0e0dd7d53e5"], [["last_selected_mode_id_on_web"]]]` | user prefs / last mode |

### `o30O0e` Request Shape

```json
[
  ["me"],
  [
    [
      ["person.photo", "person.name", "person.email"],
      null,
      [1, 7]
    ]
  ]
]
```

The first element `["me"]` requests the signed-in user's own profile. The
second element lists the person fields to return: `person.photo`,
`person.name`, `person.email`. The `null` and `[1,7]` slots are opaque
frontend markers and should be preserved exactly.

### `L5adhe` Read Request Shape

```json
[
  [null, null, null, null, null, null, null, "cf41b0e0dd7d53e5"],
  [["last_selected_mode_id_on_web"]]
]
```

The first array has the user's current mode id at a fixed index (index 7 in
the captured sample). The second array contains the preference key to read:
`"last_selected_mode_id_on_web"`.

For a read operation the first array carries the current value; the server
returns the stored preference.

### `L5adhe` Write Request Shape

To set the preference, the inner payload becomes:

```json
[
  [null, null, null, null, null, null, null, "{new_mode_id}"],
  [["last_selected_mode_id_on_web"]]
]
```

where `{new_mode_id}` is the value passed to `set_last_selected_mode`.

### Expected Response Shapes

Batchexecute responses for these RPCs follow the standard WIZ framing:

```text
)] } '

[["wrb.fr","o30O0e","[...]",null,null,null,"generic"]]
```

```text
)] } '

[["wrb.fr","L5adhe","\"{mode_id}\"",null,null,null,"generic"]]
```

The inner payload string at index 2 contains:

- For `o30O0e`: a JSON object or array describing the user's profile. Example:
  ```json
  {
    "name": "Jane Doe",
    "photoUrl": "https://example.com/photo.jpg",
    "email": "jane@example.com"
  }
  ```
  Each field is optional and may be `null` or omitted.

- For `L5adhe`: a JSON string (quoted) containing the last selected mode id, or
  `null` if unset.

## Parsing Strategy

- Strip the anti-XSSI prefix using `crate::proto::strip_xssi_prefix`.
- Locate the RPC entry by matching `["wrb.fr", "{rpc_id}", ...]`.
- Extract the inner payload string at index 2 or 3 (tolerating the extra
  wrapping level observed for batchexecute).
- For `o30O0e`: parse the inner string as `serde_json::Value`, then attempt to
  read `name`, `photo_url`, and `email` with `Option` fallback for any missing
  or null entry.
- For `L5adhe`: parse the inner string as `serde_json::Value` and, if it is a
  string, treat it as the mode id.
