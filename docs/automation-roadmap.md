# Automation Roadmap

This document describes a practical path from a research toolkit to a more product-like macOS QQ export application.

The core principle is:

> automate as much as possible, but keep privacy-sensitive actions explicit, local, and user-confirmed.

## Product Direction

The long-term target is not just a collection of scripts.  
The target is a **local-first Mac application** that:

- detects QQ data automatically
- performs safe read-only validation
- helps the user extract and verify a candidate database key
- exports chats with as few manual steps as possible
- still requires explicit confirmation for sensitive actions

## User Experience Principles

### Automate by Default

The tool should automatically handle:

- detecting the active `nt_db` directory
- checking whether databases are present
- checking WAL / malformed page conditions
- validating a candidate key
- exporting per-conversation CSV files

### Require Explicit Consent for Sensitive Steps

Sensitive actions should never be hidden inside a single opaque “Start” button.

Examples:

- attaching to QQ processes
- inspecting runtime memory
- reading active local databases
- exporting personal chat history

The application should:

1. list each sensitive action
2. let the user confirm each one deliberately
3. show a second “Are you sure?” confirmation before execution
4. clearly say that the operation stays local

## Phase Plan

## Phase 1: Research Toolkit Hardening

Current focus:

- stronger database validation
- more resilient per-conversation export
- wrapper inspection tooling
- better documentation

Deliverables:

- `validate_key`
- `inspect_wrapper`
- export diagnostics
- research docs

## Phase 2: Assisted Extraction

Goal:

turn manual key extraction into a guided, local, consent-based workflow.

Possible components:

- process detection for QQ
- wrapper path detection
- candidate marker discovery
- candidate key capture hooks
- automatic read-only verification of captured keys

Important boundary:

- do not silently extract secrets
- do not auto-run sensitive hooks without user confirmation

## Phase 3: Tauri Desktop App

Goal:

wrap the core logic into a Mac app that regular users can actually run.

Suggested first screens:

1. Welcome / local-first explanation
2. Permission and sensitive-action checklist
3. Scan / detect databases
4. Validate / assisted extraction
5. Export
6. Diagnostics / recovery

## Phase 4: Product Polish

Goal:

reduce friction without weakening consent.

Examples:

- better error messages
- better malformed-page recovery
- progress indicators
- export presets
- packaging as a distributable macOS app

## Immediate Backlog

Short-term items that move the project forward without overcommitting:

1. `validate_key` CLI
2. better wrapper inspection output
3. automatic db state reporting
4. exported diagnostics summary
5. Tauri-ready core boundary design

## Explicit Non-Goals for Now

- fully silent secret extraction
- background cloud processing
- remote upload of user data
- automatic pushing of exports anywhere

## Success Criteria

The project is moving in the right direction if:

- users only do a few important manual steps
- each sensitive step is explicit and local
- candidate keys are automatically verified, not just guessed
- exporting feels reliable even on imperfect databases
