# Spike Conventions

Patterns and stack choices established across spike sessions. New spikes follow these unless the question requires otherwise.

## Stack

- **Analysis language:** Python 3 (standard library + `json`, `urllib.parse`) for HAR parsing and protocol comparison.
- **Project language:** Rust 2021 (`cargo`) for the SDK implementation.
- **HTTP inspection source:** MITM HAR captures (`~/mitm.har`) treated as read-only evidence.

## Structure

- Spike artifacts live under `.planning/spikes/NNN-descriptive-name/`.
- Each spike contains `README.md` with YAML frontmatter (spike, name, validates, verdict, related, tags).
- Supporting machine-readable artifacts use `*.json`.
- The spike index is `.planning/spikes/MANIFEST.md`.

## Patterns

- Compare captured request/response shapes with SDK source before declaring coverage.
- Exclude analytics, ads, static JS bundles, and browser-internal endpoints from API coverage analysis.
- Use `source-path` and `rpcids` to group `batchexecute` calls.
- Distinguish core chat flow from UI telemetry/settings/history endpoints.
- Document drift (e.g., header constants) even when it does not block functionality.

## Tools & Libraries

- `python3 json.load(...)` for HAR ingestion.
- `cargo check` to verify the Rust project still compiles after source inspection.
- Git commits use `docs(spike-NNN): [VERDICT] — summary` format.
