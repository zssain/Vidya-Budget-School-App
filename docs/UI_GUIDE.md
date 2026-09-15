# UI_GUIDE.md — look, wording, translation

The prototype `reference/VidyaSchoolApp_step1.html` defines the look. Copy its CSS tokens and components; do not redesign.

## Design tokens (src/styles/tokens.css)
Copy the `:root` variables from the prototype exactly: blues, orange, green, red, purple, ink scale, line colours, radius, shadows, font stack (system fonts + Noto Sans Devanagari), `--nav`, `--top`.

## Layout
- Desktop: left navigation (230 px) + top bar + scrolling content, max width 1180 px.
- Below 820 px wide and on Android: bottom tab bar with up to 4 items + More.
- Minimum touch target 46 px. Attendance cards at least 58 px tall.

## Components to build once (src/components/)
button (primary, outline, quiet, ok, warn, small, large), input, select, chip bar, stat card, card with header, table, empty state, note (blue, green, orange, red), pill, modal, confirm dialog, toast, receipt preview, attendance card, marks input, progress bar, credential slip.

## Safe HTML
`src/core/html.js` exports:
- `html` tagged template: escapes every interpolated value (`&<>"'`), joins arrays, and returns a `SafeHtml` object.
- `raw(safeHtml)` for nesting results of `html`.
- `render(el, safeHtml)` sets `el.innerHTML`.
Never assign `innerHTML` anywhere else. ESLint rule `no-restricted-properties` blocks `innerHTML` and `outerHTML` outside `core/html.js`.

## Events
`src/core/dom.js` exports `delegate(root, { actionName: (event, element) => {} })` reading `data-action` and `data-*` attributes. No inline handlers.

## Wording rules
- Sentence case everywhere. Short, plain words. Active voice.
- Buttons say what happens: "Save attendance", "Collect fee", "Print 2 copies".
- The same action keeps the same name in button, toast and activity log ("Receipt cancelled").
- Errors say what is wrong and how to fix it: "Parent mobile must be 10 digits starting with 6, 7, 8 or 9."
- Empty screens say what to do next.
- Money: `₹1,00,000` (Indian grouping). Dates: `15 Sep 2026`. Times: `2:30 pm`.
- Use terms from PRODUCT.md glossary.

## Translation
- `src/locales/en.json` and `hi.json`, flat keys grouped by area: `nav.fees`, `fees.collect.button`, `fees.error.over_balance`.
- Parameters with `{name}`: `"fees.error.over_balance": "That is more than the balance of {balance}."`. Money and dates are formatted before insertion.
- Rust has the same keys in `crates/vidya-core/locales/{en,hi}.json` for error messages, loaded with `include_str!`.
- `scripts/check-i18n.mjs` fails if a key exists in one language only, or if a view contains quoted English text not passed through `t()` (allow-list in the script for CSS classes and data attributes).
- Hindi style: simple everyday school Hindi (उपस्थिति, फ़ीस, रसीद, अंक, कक्षा, छात्र, अभिभावक). Keep digits as 0–9.

## Accessibility
Visible keyboard focus, labels for every input, `aria-live` for toasts, colour never the only signal (text labels on pills), respect `prefers-reduced-motion`.

## Print
Documents are HTML templates in `src/print/` using `print.css` (A4, A5, 80 mm). See P5.1 and P5.2 for how they become PDFs or print jobs.
