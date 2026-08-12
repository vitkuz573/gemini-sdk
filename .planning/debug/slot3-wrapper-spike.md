---
status: investigating
trigger: "Reverse-engineer BotGuard token to StreamGenerate slot 3 transform"
created: 2026-08-12T10:20:00Z
updated: 2026-08-12T10:20:00Z
---

## Current Focus

hypothesis: "Slot 3 is a JSPB/protobuf wrapper around a BotGuard snapshot; the snapshot mixes prompt hash (qh), conversation id, request ids, and the raw token."
test: "Locate f.req builder and snapshot() call in /tmp/bard_all.js; parse slot3 bytes to reverse field layout."
expecting: "Find class that builds 97-slot array and token-manager .snapshot() usage."
next_action: "Search /tmp/bard_all.js for StreamGenerate array builder and snapshot calls."

## Symptoms

expected: "Raw BotGuard token plus metadata should reproduce captured slot 3 bytes exactly."
actual: "Raw BotGuard token is longer than slot 3 and only matches first ~10 bytes; some wrapper/transform missing."
errors: []
reproduction: "Compare /tmp/botguard_a_tokens.json tokens to slot3_422.bin / slot3_484.bin."
started: "always broken — wrapper not yet understood"

## Eliminated

## Evidence

## Resolution

root_cause: 
fix: 
verification: 
files_changed: []
