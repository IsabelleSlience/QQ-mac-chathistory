# Tauri Core Architecture

This document sketches a Tauri-oriented architecture for turning the current toolkit into a local-first macOS desktop product.

The guiding idea is:

- keep the risky and stateful logic in a Rust core
- keep the UI responsible for explanation, confirmation, and orchestration

## Why Tauri

For this project, Tauri is a reasonable fit because:

- the existing core is already Rust-heavy
- database access and validation belong in Rust
- local-first tooling benefits from a smaller runtime footprint
- sensitive operations are easier to reason about when core logic stays in one native layer

## Layer Split

## 1. Core Layer

Rust crate responsibilities:

- locate QQ data directories
- inspect wrapper paths
- validate database keys
- query conversations
- export CSV
- produce diagnostics
- define typed results for the UI

The core layer should be usable from:

- CLI tools
- Tauri commands
- tests

## 2. Tauri Command Layer

Thin command wrappers:

- `detect_db_root`
- `inspect_wrapper`
- `validate_key`
- `scan_db_health`
- `query_conversation`
- `export_csv`

This layer should not contain export logic or research logic itself.  
It should only translate between the UI and the core layer.

## 3. UI Layer

The UI should focus on:

- explaining what is happening
- showing what is sensitive
- collecting explicit confirmations
- presenting progress and results

Suggested screens:

1. Intro / local-only explanation
2. Sensitive-action checklist
3. Database detection and state
4. Key validation / assisted extraction
5. Export target selection
6. Export result and diagnostics

## Suggested Core Modules

Over time, the current crate can be reorganized into modules such as:

- `paths`
  - detect `nt_db`
  - detect `wrapper.node`
- `db`
  - open encrypted db
  - list candidates
  - health checks
- `validation`
  - validate key
  - summarize accessible tables
- `export`
  - per-conversation export
  - naming rules
  - diagnostics
- `research`
  - wrapper marker scanning
  - future assisted extraction building blocks
- `types`
  - shared serializable results for CLI and Tauri

## Sensitive Actions Model

The UI should model sensitive operations explicitly.

Example checklist:

- read local QQ database files
- inspect QQ application files
- attach to a running QQ process
- export personal chat history

Each item should map to a command or action in the core.

## Verification-First Strategy

Whenever the product reaches the “candidate key obtained” stage, the next step should be:

1. validate against `nt_msg.db`
2. verify table availability
3. verify known conversation ranges
4. only then allow export

This keeps the UI honest and reduces misleading “success” states.

## Why CLI Still Matters

Even after a Tauri app exists, the CLI should remain a first-class interface.

Reasons:

- easier testing
- easier debugging
- easier regression checks across QQ versions
- easier CI for non-sensitive paths

The Tauri app should be a front-end over a stable core, not a separate reimplementation.
