# AI Agent Guide

This directory contains instructions for AI coding agents (Claude Code, Codex, etc.) working in this repository.

## Before Starting Work

Read the following documents in order:

1. **This file** — orientation and rules
2. **`docs/ai/frontend-boundaries.md`** — frontend code placement rules
3. **`docs/ai/frontend-patterns.md`** — frontend reactive state and async patterns
4. **`docs/frontend-architecture.md`** — full frontend architecture reference
5. **`docs/api.md`** — API route system and implementation checklist
6. **`docs/auth.md`** — authentication and authorization system
7. **`docs/database.md`** — database schema conventions and Rust DB struct patterns
8. **`docs/tooling.md`** — development tools, database workflow, linting
9. **`docs/scheduling.md`** — cron triggers, DO alarms, polling loops

Only consume business or artwork docs outside this repo when explicitly asked or when product context is needed for a task.

## Repository Structure

All code changes go into `MONOREPO`.

The parent `GROUPSHOP` folder contains non-code areas such as artwork and business docs. If the task is clearly implementation work, this repo is the default place to make the change.

If there is any ambiguity about which codebase to modify, ask.

## Key Packages

| Package | Path | Role |
|---|---|---|
| `groupshop-backend-shared` | `packages/backend/backend-shared` | Route enums, traits, request/response types, and shared backend helpers |
| `api` | `packages/backend/api` | Backend handlers, DB access, durable objects |
| `frontend-shared` | `packages/frontend/frontend-shared` | Theme, atoms, API client, shared utilities |
| `landing` | `packages/frontend/landing` | Marketing/landing site |
| `migrations` | `cloudflare/api/db/migrations` | D1/SQLite schema (edit in place during dev) |

## Core Principles

- **Narrow ownership**: put code in the narrowest place that can own it correctly
- **Compile-time sync**: route types, handlers, and frontend callers should stay connected through shared Rust types instead of parallel copies
- **Theme tokens in shared**: hardcoded colors, gradients, shadows belong in `frontend-shared/src/theme/`, not in app code
- **App-local composition**: page layouts, sections, and renderers stay in their app crate unless genuinely reused by 2+ apps
- **Edit migrations in place** during development, then run `task db:recreate-dev`
- **Keep seed data in sync** with Rust enums (`repr(u8)` discriminants must match SQL `id` values)

## Writing New AI Skills / Instructions

When creating new AI-facing documentation:

1. Place the file in `docs/ai/` with a lowercase-kebab filename (e.g. `docs/ai/new-topic.md`)
2. Add an entry to the doc list in this file's "Before Starting Work" section
3. The root `CLAUDE.md` and `AGENTS.md` both point here — no need to update them for new skills
4. Keep instructions portable: no absolute paths, no user-home references, no machine-specific locations

## Verification

After completing work, run:

```sh
task lint
```

This runs formatting checks and compile checks across the current packages. Iterate until it passes with zero errors.
