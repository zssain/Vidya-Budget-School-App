# AGENTS.md — frontend (src/)

Read the root `AGENTS.md` first. These rules add to it.

## Structure (do not add other top-level folders under src/)
- `main-desktop.js`, `main-mobile.js` — entry points
- `core/` — `html.js` (safe templates), `dom.js` (event delegation), `router.js`, `ui.js` (toast, modal, confirm), `format.js` (money, dates), `i18n.js`
- `api/` — `commands.js` (the only place that calls Tauri `invoke`), `errors.js`
- `views/` — one file per screen, each exporting `render(root, params)` and nothing that touches data directly
- `components/` — reusable pieces (tables, chips, stat cards, forms)
- `styles/` — `tokens.css`, `base.css`, `components.css`, `print.css`
- `locales/` — `en.json`, `hi.json`

## Rules
- Build HTML only with the `html` tagged template from `core/html.js`. It escapes every value. Use `raw()` only for trusted, already-built `html` results.
- No inline event handlers (`onclick=` and similar). Use `data-action="name"` plus one delegated listener per view.
- No business rules: no fee maths, no permission decisions, no validation beyond "field is empty" hints. Rust decides; the UI shows the result.
- Hide buttons the user cannot use, for a clean screen, but never rely on hiding for security.
- Every string the user sees goes through `t('key', params)`. Add the key to both `en.json` and `hi.json`.
- Every command call handles `AppError` and shows `error.message` with `ui.toast` or inline next to the field (`error.field`).
- Keep the prototype's look. Tokens and component styles are in `docs/UI_GUIDE.md`.
- Tests: Vitest with jsdom for `core/` helpers and for each view's rendering with a mocked `api/commands.js`.
