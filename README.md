# Groupshop

See [COPILOT Vetting](./docs/COPILOT-VETTING.md) for the Colloseum Copilot Vetting results.

Specifically:

```
Strengths for judging:
  - Unique in the Colosseum corpus — no prior submission has combined group buying + real fulfillment
  - Composes well with Solana's strongest narrative (stablecoin payments, real-world commerce)
  - Solves a real consumer problem with a clear value prop
  - The "no tracks" structure favors novel cross-cutting ideas over category-optimized submissions
``` 

# Status

WIP - building for the Colosseum Hackathon.

----

# Developer Guide

This assumes you have Solana and Cloudflare Workers configured locally as well as all the prerequisite tooling for this repo (Rust, Taskfile, Solana, etc. etc.)

## Startup dev environment 

In one terminal, run:

```bash
task dev
```

This will start everything - the backend, build watchers to recompile programs, etc.

You'll need to keep that terminal open and run all other commands in a different terminal. To kill the dev environment, just Ctrl+C that terminal.

## Deploying 

To deploy Solana programs locally, run:

```bash
task solana-programs:deploy-local
```
