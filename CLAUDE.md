# Groupshop Claude Guide

This repository root is the software project workspace for Groupshop.

## Start Here

Read [docs/ai/rules.md](docs/ai/rules.md) before making code changes. It is the canonical AI-agent guide for this repo.

Then follow the referenced docs in that file, especially:

1. `docs/ai/frontend-boundaries.md`
2. `docs/ai/frontend-patterns.md`
3. `docs/frontend-architecture.md`
4. `docs/api.md`
5. `docs/auth.md`
6. `docs/database.md`
7. `docs/tooling.md`

## Repo Scope

- All programming work for this project belongs in this repository.
- The parent `GROUPSHOP` folder contains non-code areas such as artwork and business docs; do not move application code there unless explicitly asked.
- If a task is ambiguous, prefer this repo over sibling workspace directories.

## High-Signal Rules

- Keep code in the narrowest correct owner.
- Frontend page composition stays app-local; shared crates own primitives, tokens, and truly cross-app infrastructure.
- Hardcoded visual tokens belong in `packages/frontend/frontend-shared/src/theme/`.
- During development, edit D1 migration files in place and recreate the dev database instead of adding throwaway migrations.
- Keep SQL seed IDs and Rust enum discriminants in sync.
- Follow the compile-time route wiring pattern across shared types, backend handlers, and frontend callers.

## Verification

Run `task lint` after changes unless the user asks for a docs-only update.
