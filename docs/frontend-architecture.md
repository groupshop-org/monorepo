# Frontend Architecture

This document defines the default placement rules for frontend code in `MONOREPO/packages/frontend`.

## Core Rule

Put code in the narrowest place that can own it correctly.

- If it is used by one app, keep it in that app.
- If it is shared by multiple apps, move it to `frontend-shared`.
- Do not extract code into `frontend-shared` just because it looks reusable.
- Do not extract for hypothetical reuse.

## Decision Order

Evaluate frontend placement in this order:

1. Is it specific to one route, page, or app?
2. Is it a renderer/composition helper or a primitive/token?
3. Is there already a shared primitive that covers the styling part?
4. Do two or more apps need the same abstraction right now?

The default should be to keep code app-local until those questions clearly justify extraction.

## Ownership

`frontend-shared` owns:

- design tokens and theme primitives
- cross-app visual treatments such as `color`, `typography`, `chrome`, and `backdrop`
- reusable atoms and low-level utilities
- infrastructure that is genuinely cross-app

App crates such as `landing` or future product apps own:

- page composition
- app-specific layouts and section renderers
- branded content and copy
- route-local helper modules
- app-specific icons, SVGs, and assets

In practice, modules like `layout`, `sections`, `hero`, `content`, and `header` should usually start inside the app crate.

## Theme Rules

Theme modules should expose semantic UI treatments, not page-specific renderers.

Good theme module names:

- `color`
- `typography`
- `chrome`
- `backdrop`

Bad theme module names:

- `site`
- `auth`
- `marketing-page`
- `deal-home`
- route-specific names

If a module starts to describe one app, one page, or one route, it probably does not belong in shared theme.

If a shared module mostly exists to return `Dom` for one product surface, it probably belongs in an app crate.

## Layout Rules

When building pages, prefer app-local modules such as:

- `layout`
- `sections`
- `rendering`
- `header`
- `hero`

These modules should live next to the pages that use them unless there is proven cross-app reuse.

## Extraction Test

Before moving code into `frontend-shared`, confirm all of the following:

1. At least two apps need the same abstraction.
2. The abstraction can be named without referring to a specific app, route, or page.
3. The extracted API is simpler than leaving the code local.
4. The extracted module will not immediately become a grab-bag for unrelated UI.

If any of those fail, keep the code app-local.

If the extracted API needs words like `landing`, `auth`, `marketing`, `hero`, or a route name, that is a strong sign the abstraction is not actually shared.

## Do Not Extract Yet

Do not move code into `frontend-shared` when it is:

- only used by one app today
- mostly content plus styling
- a convenience renderer for one page section
- a wrapper created only to reduce small amounts of repetition
- a layout helper whose callers all live in one app

## Page-Owned Shells

If a screen visually reads as one shell, keep that shell in the page or app-local layout module.

- Do not split a single visual frame across the global app shell and a page-local shell unless that split is actually reused.
- Logged-out or special-case surfaces can own their own brand bar, canvas, divider, and footer when those pieces read as one composition.
- The global shell should step aside for those surfaces rather than forcing every page through the same frame.

## Styling Rules

- Hardcoded colors, gradients, shadows, and similar visual treatment tokens belong in shared theme primitives.
- App-local layout modules may consume those tokens, but should not define raw visual values unless the value is truly app-specific and asset-like.
- Shared tokens should be named by treatment or role, not by page name.
- Prefer extracting the token before extracting the renderer.
- Shared theme should provide treatments; app crates should assemble product-specific UI.
- If something looks interactive, it should usually get interactive affordance styling even before behavior is wired.
- Button-like controls should usually have both `cursor: pointer` and disabled text selection (`user-select: none`).
- If a control visually reads as one clickable row, the whole row should usually own the interaction rather than only a small sub-element.

## Form Rules

- If a page is visually representing a real email/password form, prefer real form controls instead of decorative shells.
- Use browser-facing semantics such as `name`, `type`, and `autocomplete` so Chrome/password managers can autofill or suggest values correctly.
- Keep page-specific form layout and validation rendering in the app crate unless multiple apps truly need the same form abstraction.
- If autofill styling needs dark-theme overrides, use valid CSS and shared theme tokens rather than ad hoc page-local color values.

## Validation Rules

- Inline validation and status surfaces that belong to one page should stay in that page's local layout/rendering module.
- Use shared semantic error tokens for text, borders, and fills instead of introducing page-local red values.
- Keep the state and message selection in the page, and let layout helpers only render the surface.

## External URL Rules

- Never hardcode production URLs. All cross-app URLs must come from build-time environment variables via `required_build_env!()` and the app's `Config` struct.
- Each frontend app declares its external URLs in `config.rs`.
- Dev values are wired in the app's taskfile using variables from `taskfiles/config.yml`.
- If you add a link to another app or external service, check that the URL comes from config, not a string literal.

## CTA Hierarchy

- Keep a clear visual hierarchy between the primary action and route-switch or helper actions.
- Primary submit or auth actions may use filled treatments.
- Secondary navigation actions such as route-switch buttons should usually use a quieter outline treatment so they do not compete with the main submit CTA.
- Helper actions like “Forgot password” should typically match that quieter secondary treatment.

## Naming Guidance

- Shared theme: use semantic names like `chrome`, `backdrop`, `color`, `typography`.
- App-local modules: use structural names like `layout`, `sections`, `hero`, `content`, `header`.
- Avoid names that import page intent into shared crates.

## Working Pattern

When adding a new page or site:

1. Start in the app crate.
2. Build page layout and render helpers locally.
3. If raw visual values appear, first ask whether they are theme tokens.
4. Move only the tokens or truly shared primitives into `frontend-shared`.
5. Leave page-specific renderers in the app unless another app clearly needs them.
6. For visually self-contained screens, decide whether the full shell should stay page-owned.
7. For real forms, verify browser semantics like autofill/password-manager support before treating the work as done.
8. Re-check the final module names and remove app-specific language from anything shared.

## Review Checklist

Before finishing a frontend refactor, ask:

1. Did page composition stay in the app crate?
2. Did shared code stay primitive and semantic?
3. Does any shared module name accidentally describe one app or route?
4. Did I extract because of real reuse, not because the code looked tidy in shared?
5. Would a new engineer understand ownership just by looking at the module names?
6. Does the screen have one visual shell that should remain page-owned?
7. Do interactive-looking controls have the right affordance styling, including pointer cursor and disabled text selection where appropriate?
8. If this is a real form, did I use real inputs and the correct browser autofill semantics?
9. Are inline validation surfaces page-local while their colors/treatments come from shared semantic tokens?
10. Do secondary route-switch CTAs stay visually quieter than the primary action?
11. Do all cross-app URLs come from `Config` / `required_build_env!()`, not hardcoded strings?
