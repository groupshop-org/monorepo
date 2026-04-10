# Scheduling

This document tracks periodic, delayed, or polling behaviour in Groupshop.

## Current State

As of now, the repo does not define any scheduled production workflows beyond the normal request-driven API and frontend dev tasks.

Specifically:

- no Cloudflare cron triggers are configured in the checked-in worker config
- no documented Durable Object alarm workflows are implemented in this repo yet
- no dedicated health worker or health dashboard exists in this codebase

## Why Keep This Doc

The product will likely grow scheduled behaviour later, for example:

- expiring stale deal windows
- refund timeouts
- cleanup of one-time auth tokens
- delayed fulfillment-state reconciliation
- periodic catalog refresh or supplier sync

When that happens, record the mechanism here.

## Scheduling Primitives To Use

When you add scheduled behaviour, choose the narrowest primitive that fits:

| Primitive | Runtime | Use Case |
|---|---|---|
| Cloudflare Cron Triggers | Worker | fixed-interval tasks |
| Durable Object Alarms | Durable Object | delayed one-shot work tied to one logical object |
| App-side polling | Frontend | UI refresh for user-visible state |

## Documentation Rules For Future Additions

When introducing scheduled work:

1. Document where it is configured.
2. Document the code path that handles it.
3. Document what state it reads or mutates.
4. Document how local development exercises it.
5. Update this file in the same change.
