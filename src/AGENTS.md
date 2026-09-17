# AGENTS.md — frontend (src/)

Read the root `AGENTS.md` first. These rules add to it.

## Structure (do not add other top-level folders under src/)
- `main-desktop.jsx`, `main-mobile.jsx` — entry points
- `core/` — providers, hooks, `router.jsx`, `useCommand.js`, `format.js`, `i18n.jsx`
- `api/` — `commands.js` (the only place that calls Tauri `invoke`), `errors.js`
- `views/` — one `.jsx` component per screen; views never touch data directly
- `components/` — reusable `.jsx` components (tables, chips, stat cards, forms)
- `styles/` — `tokens.css`, `base.css`, `components.css`, `print.css`
- `locales/` — `en.json`, `hi.json`

## Rules
- Render text only as JSX children or attribute values. React escapes it.
- Never use `dangerouslySetInnerHTML`, `innerHTML`, `outerHTML`, `insertAdjacentHTML` or `document.write`.
- Pass event handlers only as React props.
- Components get data from hooks in `core/useCommand.js`; they never call `invoke` directly.
- No business rules: no fee maths, no permission decisions, no validation beyond "field is empty" hints. Rust decides; the UI shows the result.
- Hide buttons the user cannot use, for a clean screen, but never rely on hiding for security.
- Every string the user sees goes through `t('key', params)` from `useT()`. Add the key to both `en.json` and `hi.json`.
- Every command call handles `AppError` and shows `error.message` with `ui.toast` or inline next to the field (`error.field`).
- Keep the prototype's look. Tokens and component styles are in `docs/UI_GUIDE.md`.
- Tests: Vitest and React Testing Library for `core/` hooks, components and every view with mocked `api/commands.js`.
