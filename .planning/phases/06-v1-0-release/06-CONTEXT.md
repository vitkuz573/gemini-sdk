# Phase 6: v1.0 Release - Context

**Gathered:** 2026-08-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Polish documentation, verify semver, and prepare the crate for v1.0 publication. Scope covers TOOL-05.

Key outcomes:

- Final API audit and deprecation cleanup.
- MSRV policy documented and verified.
- crates.io publication readiness with changelog and release notes.
- Migration guide from v0.x to v1.0.

</domain>

<decisions>
## Implementation Decisions

### Version Policy
- Keep crate version at `0.1.0` for this milestone (v0.1 Core milestone).
- Add a CHANGELOG.md that documents all changes from initial state to v0.1.0 and notes the path toward v1.0.
- Do not bump to `1.0.0` now; the ROADMAP explicitly labels this milestone as v0.1 Core.

### crates.io Publication Readiness (TOOL-05)
- Run `cargo publish --dry-run` to verify the manifest packages correctly.
- Ensure `Cargo.toml` has all required fields: name, version, authors, edition, license, description, repository, readme, keywords, categories, rust-version.
- Ensure LICENSE file is present and correct.
- Ensure README.md is accurate and reflects current features.
- Document the exact `cargo publish` command for the user to run later.
- Do not perform a real publish; credentials are not available and release control belongs to the user.

### API Audit and Deprecation Cleanup
- Run `cargo public-api` if available; otherwise do a manual review of `src/lib.rs` re-exports.
- Remove any leftover `#[allow(unused)]` or dead code introduced during development.
- Ensure all public items have doc comments and `#![warn(missing_docs)]` passes cleanly.
- Verify no public items expose internal protocol details unintentionally.

### MSRV Verification
- Verify `rust-version = "1.80"` in Cargo.toml.
- Document MSRV policy in README.md and CONTRIBUTING.md.
- Run tests with the installed toolchain; note if a toolchain check is not possible in this environment.

### Documentation and Release Notes
- Write CHANGELOG.md with sections for each phase (v0.1.0 release notes).
- Update README.md to mention new features: hooks, tracing, injectable client, upload progress, audio/video, tools, metrics, session save/restore.
- Add a v0.1 → v1.0 migration guide section in docs/ or README.md noting breaking changes (async config builders, attestation errors).

### the agent's Discretion
- Agent may adjust changelog format and migration guide location.
- Agent may add or update examples to reflect new features.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Cargo.toml` — manifest with version 0.1.0, features, metadata.
- `README.md` — project overview and installation.
- `LICENSE` — MIT license.
- `src/lib.rs` — public API re-exports.
- `docs/protocol.md` — protocol documentation.
- Phase summaries in `.planning/phases/*/0*-SUMMARY.md`.

### Established Patterns
- Public API explicitly re-exported in `src/lib.rs`.
- Examples declared in Cargo.toml.
- Tests use inline modules and integration tests.
- Documentation comments required on public items.

### Integration Points
- CHANGELOG.md → references phase summaries and commits.
- README.md → updated feature list and usage examples.
- Cargo.toml → version and metadata unchanged.

</code_context>

<specifics>
## Specific Ideas

- Include a "Release checklist" in CHANGELOG.md or a new RELEASE.md with the cargo publish command.
- Mention breaking changes explicitly: `with_language`/`with_max_retries`/`with_timeout` are now async; `Error::AttestationFailed` replaces silent fallback.
- Add a note that v1.0 will bump to `1.0.0` and follow semver strictly.

</specifics>

<deferred>
## Deferred Ideas

- Real crates.io publish deferred to user action after this phase.
- Version bump to 1.0.0 deferred until the v1.0 milestone is actually ready.

</deferred>
