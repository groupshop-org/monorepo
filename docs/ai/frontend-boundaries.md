# Frontend Boundaries

Use this guide when editing or refactoring frontend code in `packages/frontend`.

For the full human-readable architecture reference, see `docs/frontend-architecture.md`.

## Priority Order

Apply these questions in order:

1. Is this specific to one route, page, or app?
2. Is this a structural renderer or a visual primitive?
3. Is there already a shared primitive that solves the styling part?
4. Do at least two apps need the same abstraction today?

The default answer should bias toward app-local code.

## Default Rule

Put code in the narrowest place that can own it correctly.

- One app: keep it in that app.
- Multiple apps: consider `frontend-shared`.
- "Looks reusable" is not enough by itself.
- Do not extract for hypothetical future reuse.

## Shared Ownership (`frontend-shared`)

- Theme tokens and primitives
- Cross-app visual treatments (`color`, `typography`, `chrome`, `backdrop`)
- Reusable atoms and utilities
- Infrastructure used by multiple frontend apps

## App Ownership (`landing`, future product apps, etc.)

- Page composition
- App-specific layout/rendering helpers
- Page sections and route-local helpers
- Branded content
- App-specific SVGs and assets

## Hard Split

Keep these app-local unless there is real cross-app reuse:

- `layout`, `sections`, `hero`, `content`, `header`
- Route-specific render helpers
- Marketing-site composition
- Deal-specific cards and auth surfaces
- Page-local validation banners and consent rows

Keep these shared when used across apps:

- `color`, `typography`, `chrome`, `backdrop`
- Low-level atoms
- Low-level utilities

## Theme Guidance

Shared theme modules should be semantic, not page-specific.

Good names: `color`, `typography`, `chrome`, `backdrop`

Bad names: `site`, `auth`, `marketing`, `landing-page`, `deal-home`

If a shared module name refers to one app, page, or route, stop and reconsider.

## Extraction Test

Only move code into `frontend-shared` when all are true:

1. At least two apps need it.
2. It can be named without app/page language.
3. The extracted API is cleaner than leaving it local.
4. The result is a real primitive, not a grab-bag.

## Styling Rules

- Hardcoded colors, gradients, shadows belong in shared theme primitives.
- App-local layout modules consume those tokens but do not define raw visual values.
- Prefer extracting the token before extracting the renderer.
- If something looks interactive, give it interactive affordance styling (`cursor: pointer`, `user-select: none`).

## Form Rules

- Real forms should use real form controls with proper `name`, `type`, and `autocomplete` attributes.
- Keep page-specific form layout in the app crate.
- Dark-theme autofill overrides should use shared theme tokens, not ad hoc values.

## External URL Rules

- Never hardcode production URLs. All cross-app URLs must come from build-time environment variables via `required_build_env!()` and the app's `Config` struct.
- Each frontend app declares its external URLs in `config.rs`.
- Dev values are wired in the app's taskfile using variables from `taskfiles/config.yml`.
- If you add a link to another app or external service, check that the URL comes from config, not a string literal.

## CTA Hierarchy

- Primary actions: filled treatments.
- Secondary navigation (route-switch): quieter outline treatment.
- Helper actions ("Forgot password"): match the quieter secondary treatment.

## Working Pattern

1. Start implementation in the app crate.
2. Build page structure with app-local modules.
3. When you see raw colors/gradients/shadows, ask whether they are theme tokens.
4. Move only the token or primitive into `frontend-shared`.
5. Leave the renderer local unless another app clearly needs it.
6. For real forms, verify browser semantics before treating the work as done.

## Review Checklist

Before finishing frontend work:

1. Did page composition stay in the app crate?
2. Did shared code stay primitive and semantic?
3. Does every shared module name avoid app/route language?
4. Did I avoid introducing a catch-all shared renderer module?
5. Do interactive controls have pointer cursor and disabled text selection?
6. If this is a real form, did I use real inputs with correct autofill semantics?
7. Are inline validation surfaces page-local while their colors come from shared tokens?
8. Do secondary CTAs stay visually quieter than the primary action?
9. Do all cross-app URLs come from `Config` / `required_build_env!()`, not hardcoded strings?
