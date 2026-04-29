# qq-mac-export-tools

Research-oriented tooling for inspecting and exporting local chat history from recent macOS QQ / NTQQ databases.

This repository focuses on two things:

- **database-side analysis** for recent Mac QQ / NTQQ local storage
- **conversation-level export** into formats that are easier to inspect with tools like ChatGPT, pandas, or SQLite

It is intentionally **not** a one-click secret extractor.  
The repository is designed to be publishable, reviewable, and reusable as a research toolkit.

## What This Project Provides

- Auto-detection of common macOS QQ `nt_db` directories
- Read-only opening of encrypted NTQQ databases once you already have a valid key
- Per-conversation CSV export for direct chats and groups
- Naming rules tuned for real-world review workflows:
  - direct chats prefer **remark**
  - otherwise **nickname + QQ number**
  - groups prefer **group name + group number**
- Conversation-by-conversation export to reduce the blast radius of malformed pages
- Research helpers for `wrapper.node` inspection
- Small verification tools for checking whether a candidate key really matches a target database

## What This Project Does Not Provide

- It does **not** ship any real user key
- It does **not** claim universal compatibility with every QQ build
- It does **not** currently provide a one-click Mac key extraction implementation

That boundary is deliberate. The goal here is to keep the repository useful and reusable without turning it into a low-friction secret extraction package.

## Why CSV Instead of Excel

This project exports to **CSV** on purpose.

Compared with Excel workbooks, CSV is better for:

- ChatGPT uploads and structured review
- pandas / Python pipelines
- quick shell processing
- diffing and versioning
- per-conversation file splitting

## Repository Layout

```text
src/bin/export_latest_csv.rs    # export one CSV per conversation
src/bin/query_conversation.rs   # verify date ranges and counts for a single direct chat
src/bin/inspect_wrapper.rs      # statically inspect wrapper.node markers
src/bin/validate_key.rs         # validate a candidate key and summarize database access
src/lib.rs                      # shared database helpers
docs/macos-key-extraction.md    # key extraction notes and boundaries
docs/research-workflow.md       # end-to-end research workflow
docs/automation-roadmap.md      # path from toolkit to user-facing product
docs/tauri-core-architecture.md # Tauri-oriented app/core split
```

## Quick Start

### Build

```bash
git clone https://github.com/YOUR_NAME/qq-mac-export-tools.git
cd qq-mac-export-tools
cargo build --release
```

### Export All Conversations to CSV

```bash
cargo run --bin export_latest_csv -- \
  --key "YOUR_16_BYTE_KEY" \
  --db-root "/Users/you/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_xxx/nt_db" \
  --output "./exports"
```

If `--db-root` is omitted, the tool will try to auto-detect the latest `nt_db` directory under the current macOS user profile.

Typical output:

```text
exports/
  direct/
    Alice.csv
    Bob__123456789.csv
  group/
    Example_Group__541305724.csv
  direct_index.csv
  group_index.csv
  group_errors.csv
  README.txt
```

### Check a Single Direct Conversation

```bash
cargo run --bin query_conversation -- \
  --key "YOUR_16_BYTE_KEY" \
  u_xxxxxxxxxxxxxxxxxxxxx
```

This prints:

- total message count
- earliest timestamp
- latest timestamp
- a few sample rows for sanity-checking

### Inspect `wrapper.node`

```bash
cargo run --bin inspect_wrapper -- \
  --wrapper "/Applications/QQ.app/Contents/Resources/app/wrapper.node" \
  --verbose
```

This helper does **not** extract a runtime key for you.  
It helps you quickly see whether the current build still exposes useful markers such as:

- `nt_sqlite3_key_v2`
- `sqlcipher`
- `codec`
- `set_pass`

### Validate a Candidate Key

```bash
cargo run --bin validate_key -- \
  --key "YOUR_16_BYTE_KEY" \
  --db-root "/Users/you/Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/nt_qq_xxx/nt_db"
```

This is meant to be the first “real automation” step in the workflow.

It reports:

- which databases could be opened
- whether expected tables exist
- rough message time ranges
- whether the candidate key is likely usable for export

## Export Format

Each conversation CSV includes:

- `conversation_id`
- `conversation_number`
- `conversation_label`
- `sender_uid`
- `sender_uin`
- `is_self`
- `send_time`
- `send_time_iso`
- `msg_type`
- `sub_msg_type`
- `send_status`
- `message_summary`

## Research Docs

- [MacOS Key Extraction Notes](./docs/macos-key-extraction.md)
- [Research Workflow](./docs/research-workflow.md)
- [Automation Roadmap](./docs/automation-roadmap.md)
- [Tauri Core Architecture](./docs/tauri-core-architecture.md)

These docs are meant to help people reproduce the **analysis process**, not just consume a finished exporter.

## Current Scope

This repository has been validated against recent Mac QQ / NTQQ local database layouts in practice, but results may still vary depending on:

- QQ version
- database layout changes
- active WAL state
- malformed pages
- symbol stripping or wrapper changes

## Known Limitations

- Some malformed pages can still break specific conversations
- Group export may be less complete than direct-chat export on damaged databases
- Key extraction is still documented as a research workflow rather than packaged as a one-click implementation
- The current direction is assisted automation with explicit user confirmation, not silent secret extraction

## Roadmap

- Improve malformed-page recovery
- Improve group-chat export resilience
- Expand message-type parsing and summaries
- Add stronger wrapper inspection and reporting tools
- Continue documenting reproducible Mac QQ research workflows
- Move toward a Tauri-based local-first Mac client

## Safety Notes

- Do **not** commit your real database key
- Do **not** publish raw databases
- Do **not** push private exports containing personal chat history to a public repository

## License

[MIT](./LICENSE)
