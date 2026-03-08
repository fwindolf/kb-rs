# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-03-08

### Changed
- **Renamed project from mulch to kb** — binary is now `kb`, data directory is `.kb/`, config is `kb.config.yaml`
- Crate names: `mulch` → `kb`, `mulch-core` → `kb-core`
- All internal types: `MulchError` → `KbError`, `MulchConfig` → `KbConfig`
- HTML markers: `<!-- mulch:start -->` → `<!-- kb:start -->`

### Added
- Knowledge quality guidance across all surfaces (priming prompt, onboarding, MCP tools, CLI help, README)
- Two anti-patterns enforced: no code-discoverable content, no hardcoded file paths
- MCP integration tests (327 lines)

## [0.1.2] - 2026-02-23

### Changed
- `onboard` rewritten: static onboarding content (kb knowledge + bd task tracking)
  instead of dynamic expertise section
- `onboard` now supports target flags: `--auto`, `--agents`, `--claude`, `--copilot`,
  `--codex`, `--opencode` (mutually exclusive) with auto-discovery fallback
- `onboard --check` to verify if onboard section is installed
- `onboard --remove` to remove the onboard section (deletes file if empty)
- Removed `ensure_kb_dir` call from onboard (no longer needs config)

## [0.1.1] - 2026-02-23

### Changed
- `onboard` command now targets `CLAUDE.md` or `AGENTS.md` instead of `README.md` (prefers existing `CLAUDE.md`, falls back to `AGENTS.md`)
- Onboard section uses lightweight instructions focused on kb workflow
- Removed `--provider` flag from `onboard` (no longer needed)
- `init` no longer creates `.kb/README.md`

## [0.1.0] - 2026-02-23

### Added
- Initial Rust port of [mulch](https://github.com/jayminwest/mulch)
- Two-crate workspace: `kb-core` (library) and `kb` (CLI binary)
- 20 CLI commands: init, add, record, edit, query, search, delete, prime, status, validate, prune, doctor, ready, learn, compact, setup, onboard, sync, update, diff
- 6 record types: convention, pattern, failure, decision, reference, guide
- BM25 full-text search across domains
- Token budgeting with priority-based record selection
- JSONL storage with atomic writes and advisory file locking
- Format-level compatibility with existing `.kb/` directories from the TypeScript version
- Output formats: markdown, XML, plain text, compact, MCP JSON
- Cross-platform CI (Linux, macOS, Windows)
- Release workflow for 6 targets (Linux/macOS/Windows x86_64 + ARM64)
- 132 tests (90 integration + 42 unit)
