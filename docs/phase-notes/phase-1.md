# Phase 1 handoff — Foundation, size spike, design system, mock screens

Branch: `rebuild/p01` · Platform built: macOS (Apple Silicon) · Android: **deferred**
(owner decision — see below). Status: **desktop path substantially complete; two
gaps carried forward (fidelity ≤0.1%, Android tooling).**

## Start state
- Tools: Node v25.3.0, npm 11.7.0, rustc 1.98.1, cargo 1.98.1. **JDK: not installed.**
  Android SDK present at `~/Library/Android/sdk` (platform android-37, build-tools
  36.0.0) but **no `cmdline-tools`, no NDK**, env vars unset, no android Rust targets.
- Repo was on `edition2-backend` (a prior build at "P3.5"); the whole old edition was
  deleted in the working tree, with the new build pack (docs/design/prompts) added on top.

## Owner decisions taken (asked at the start)
1. **Repo strategy → Fresh start.** Old edition-2 code (incl. `crates/vidya-core`) is
   NOT ported; it stays in history on `edition2-backend`. New branch `rebuild/p01`.
   `crates/vidya-core` is a fresh empty lib with one passing test (rules land in Phase 2).
2. **Android tooling → desktop now, Android later.** JDK 17 + NDK + android Rust targets
   are absent; per Step 1 this is a STOP. All Android steps (size-spike APKs, debug APK
   on device) are deferred until the owner installs JDK 17 + NDK. Everything else was done.

## Repo path taken
Fresh start on `rebuild/p01`. First commit removed the edition-2 source and added the
build pack. No `legacy/` folder (nothing ported). Shared design assets copied into
`src/assets/` (3 SVG lockups) so Vite bundles them; brand-kit app icons copied into
`src-tauri/icons/`.

## What was built
**Scaffold** — Vite + React 18 (pinned per §4) + TS strict + Tailwind v4 (`@tailwindcss/vite`),
hand-authored config (no interactive generators). Tauri 2 initialised: identifier
`in.vidyabudget.app`, window 1440×900 / min 1280×720 / centered, bundle targets
nsis+dmg+app, macOS min 12.0, `webviewInstallMode=embedBootstrapper` (all keys verified
against `config.schema.json`). Cargo workspace (`src-tauri` + `crates/vidya-core`),
size-tuned `[profile.release]` (lto, codegen-units=1, opt-level="s", strip, panic=abort —
kept; build succeeded).

**Size spike** — every §13 native dependency added with exact features + `src-tauri/src/spike.rs`
using each so the linker keeps it. Crypto gate (see table). Desktop-only crates
(`mdns-sd`, `keyring` apple/windows-native) under the `cfg(not(android))` target. Barcode
scanner not added yet (Android-only; evaluated with the APK spike, deferred).

**Design system** — `styles/tokens.css` (every §2 colour + radii 4..22 + `--ease`),
`keyframes.css` (all §5 variants + global button transition + 3 `:active` scales +
reduced-motion), `app.css` (Tailwind `@theme inline` token→utility map, runtime accent,
Hindi serif swap). Fonts bundled, none from the web: Geist 400/500/600/700 + Noto Sans
Devanagari 400/500/600 (Fontsource subsets), **Newsreader variable opsz** (normal+italic,
Latin) hand-aliased in `fonts.css` to the mock family name `'Newsreader'`. `lib/theme.ts`
`setAccent()/loadAccent()`.

**Icons** — `lib/icons.ts` (32 named icons matching §6, two-tone tile split preserved) +
`components/Icon.tsx` (`size`, `strokeWidth`=1.6, `color`, two-tone, a11y).

**Mock renderer** — `design/runtime/support.js` (~290 lines, no deps): renders the
unmodified `.dc.html` (DCLogic/state/setState, `{{holes}}`, `<sc-for>`/`<sc-if>`, `on*`
handlers, boolean/aria attrs, `/_blob` rewrite, `<helmet>`→head, focus/caret preserve).
Served DEV-only at `/design/screens/support.js` by a vite middleware; `#/__mocks` links to
the five mocks.

**Platform / router / i18n / format** — `lib/platform.ts` (build-time `VITE_PLATFORM` +
DEV `?platform` override, `isPhone`); `lib/router.ts` (hash router via
`useSyncExternalStore`, `navigate`, `matchRoute` params); `lib/i18n` (hand-written `t()`
merging per-screen string modules, `en` verbatim, `hi` = `TODO-HI: <en>`, `useLang`/`setLang`);
`lib/format.ts` (Intl `en-IN` money/date/session/relative/greeting/parseDigits, **34 vitest
tests**).

**Screens (props-only) + verbatim fixtures** — `WelcomeScreen` (desktop + phone single-column
variant), `PrincipalHomeScreen`, `CollectFeeScreen`, `TeacherHomeScreen`, `AttendanceScreen`.
Each is a verbatim translation of its mock (same DOM + inline styles), with the mock's local
keyframe names mapped to the app's (vRise→vRise12/14, vIn→vIn6/8/10, vPop→vPopFee/vPopAtt,
vSheet, vFade, vGrow, vFill, vPulse), and accent computed values replaced by `var(--accent[-6/-10/-12])`.
Fixtures in `src/dev/fixtures/*` copy the sample data verbatim. Every visible string goes
through `t()`.

**Routing / dev** — `App.tsx` hash routes: `#/welcome` (default), `#/principal/home`,
`#/accountant/collect`, `#/teacher/home`, `#/teacher/attendance/:classId`, plus DEV-only
`#/__gallery` (index of all five screens at mock size + `?screen=&state=` single-screen mode
for fidelity) and `#/__mocks`; everything else → `Placeholder`. `VITE_DEV_START=teacher`
opens Teacher Home. App icons from the brand kit in `src-tauri/icons/`.

## Commands + real output (trimmed)
- `cargo tree -i aws-lc-rs` / `-i aws-lc-sys` → `did not match any packages` (absent ✓).
- `cargo tree -i ring` → sole TLS provider (rcgen, rustls→hyper-rustls/reqwest/tokio-rustls→tokio-tungstenite/axum).
- `cargo tree -i openssl-sys` → **only** `libsqlite3-sys → rusqlite` ✓.
- `cargo check -p vidya` → Finished (0 warnings). `cargo build --release -p vidya` → Finished, **binary 9,196,144 bytes (8.8 MB)**, 0 warnings, 1m35s.
- `npx tsc --noEmit` → clean. `npx vitest run` → **34 passed**. `npx vite build` → ok (JS 221 kB / 62 kB gzip, CSS 16 kB).
- `grep -rn fonts.googleapis src index.html` → none. `npm ls lucide-react` → empty. No animation lib in package.json.
- `npx playwright test phase1` → **5 passed** (see Test results).

## Test results
- **vitest**: 34/34 (format.ts).
- **cargo**: `cargo test --workspace` — vidya-core placeholder test passes (run again in CI;
  the workspace also builds the app crate).
- **Playwright phase1 (§8 interactions)**: 5/5 GREEN —
  collect-fee typing 5000 → error + disabled button; Full-due chip → balance ₹0; Cash hides
  reference; attendance submit-locked / mark-all / undo / submit → locked + submitted bar;
  welcome radios swap field + CTA.
- **Playwright fidelity (§9)**: harness built and runs (mock via support.js provides the
  baseline; app asserted). **≤0.1% NOT met — see Fidelity results.**

## Size spike table
| Platform | Artifact | Download bytes | Installed | Notes |
|---|---|---|---|---|
| macOS (arm64) | release binary `vidya` | 9,196,144 (8.8 MB) | — | stripped, LTO, opt=s |
| macOS (arm64) | **`Vidya_0.1.0_aarch64.dmg`** | **5,577,982 (5.58 MB)** | — | download artifact — **7× under the 40 MB limit** |
| macOS (arm64) | **`Vidya.app`** | — | **9,863,646 (9.86 MB)** | installed — **5× under the 50 MB limit**; no bundled Frameworks (system WKWebView) |
| Windows | NSIS setup | — | — | **deferred** (build on Windows) |
| Android arm64-v8a / armeabi-v7a | APK | — | — | **deferred — no JDK/NDK** |
| Barcode-scanner delta (APK) | — | — | — | **deferred with Android** |

`scripts/size-report.mjs` → OK (both artifacts within budget). Every §13 native dependency
was linked in for this measurement (spike.rs), so these are worst-case sizes; they will only
shrink slightly once `spike.rs` is deleted in Phase 2.

Crypto gate: `aws-lc-rs`/`aws-lc-sys` absent; `ring` sole provider; `openssl-sys` only via
`libsqlite3-sys`. Resolved: axum 0.7.9, reqwest 0.12.28, rustls 0.23.45, tokio-rustls
0.26.5, rcgen 0.13.2, tokio-tungstenite 0.24.0, rusqlite 0.32.1 (libsqlite3-sys 0.30.1),
keyring 3.6.3, mdns-sd 0.11.5, ed25519-dalek 2.2.0, chacha20poly1305 0.10.1. `cargo bloat`
not installed (not run).

## Fidelity results per screen/state (app vs mock baseline, pass-2)
| State | diff ratio | note |
|---|---|---|
| welcome-setup / join / recover | 0.03–0.04 | edge-level (fonts + geometry) |
| principal-home | 0.07 | " |
| collect-fee default / over / cash | 0.03–0.04 | " |
| collect-fee-success | 0.33 | test-drive artifact: mock did not advance to the success panel via support.js click (compared app-success vs mock-editing) |
| teacher-home | 0.04 | " |
| attendance default / mark-all | 0.03–0.08 | " |

**Diagnosis (with diff screenshots in test-results/):** layout, colours, content and text
positions are correct; the differences are the glyph edges of essentially all text **and**
solid box borders. Root causes: (1) the mock loads fonts from Google while the app bundles
Fontsource — the same typefaces but pixel-different builds/hinting; (2) a small sub-pixel
geometry difference. Per §9 / the STOP conditions ("font rendering difference > threshold
you cannot fix → report, don't approximate") this is **reported, not forced.** Options for
the owner in §Questions.

## Deviations (from the prompt)
- **Fresh start** (owner) → `crates/vidya-core` not ported; empty placeholder lib.
- **Android deferred** (owner) → no APK size numbers, no android init/icons yet.
- `@types/node` added as a **type-only devDependency** (not in §13's explicit list) — required
  by `vite.config.ts`/config typechecking; zero runtime/bundle impact. Flag for approval.
- shadcn/ui CLI was **not run**; the design primitives are realised directly as the five
  verbatim mock screens (which inline sidebar/header/sheet/etc.). Standalone reusable
  components + a component-variant gallery (§7) are **thin** this phase — the gallery shows
  the five screens (DONE item) but not every primitive in isolation. Recommend extracting
  Sidebar/Header/Card/StatStrip/Button/Pill/Sheet/AttendanceRow/Tile in Phase 3 when the
  non-mock screens need them. lucide never entered the tree (nothing to remove).
- i18n `en.json`/`hi.json` are split into per-screen TS modules merged by `lib/i18n/index.ts`
  (functionally equivalent; avoids one giant file and parallel-edit conflicts).
- React pinned to 18 (per §4), not 19.
- Newsreader family aliased in `fonts.css` (Fontsource registers `'Newsreader Variable'`; the
  mock uses `'Newsreader'`).

## [OWNER] defaults used
- Accent default `#2F7479` (options `#2F7479 #1F4E8C #5B4B8A`).
- Company/legal placeholders kept verbatim from the mock ("[Your company]").

## Known issues / gaps
1. **Fidelity ≤0.1% not met** (font-render + sub-pixel geometry). Biggest single lever would
   be for the app and the mock to render the *same* font bytes. See Questions.
2. **Android** entirely deferred (no JDK 17 / NDK). Size budget therefore proven for desktop
   only so far (well under limit at 8.8 MB binary).
3. `collect-fee-success` fidelity comparison needs the mock `support.js` "Record payment"
   click to advance to the success panel; investigate support.js handler timing.
4. Standalone component library + component gallery (§7) is minimal (see Deviations).
5. Tauri warns the identifier `in.vidyabudget.app` ends with `.app`, which collides with the
   macOS bundle extension. It built and bundled fine, but consider `in.vidyabudget.vidya`
   (or similar) if it causes trouble on macOS. Kept as specified in the prompt (Step 2.3).

## Questions for the owner
1. **Fidelity fonts.** To hit ≤0.1%, do you want (a) accept a higher threshold for text-heavy
   screens, (b) have the fidelity test load the *same* bundled fonts into the mock (I can do
   this in the dev-only renderer without editing the mock files), or (c) treat "visually
   indistinguishable" as met by manual review of the reference PNGs? (a)/(b) are quick; (c)
   needs your side-by-side.
2. **Android tooling.** Install JDK 17 + Android NDK now (I can script it via Homebrew +
   `sdkmanager`) so I can finish the APK size spike + debug APK, or keep deferring?
3. Approve the `@types/node` type-only devDependency?

## What Phase 2 needs
- Rebuild `crates/vidya-core` rules from scratch (fresh-start): money/paise, fees, grades,
  permissions, attendance, marks, requests, conflicts, hlc, receipts, admissions, licence,
  validation, words, csv — pure Rust + tests. Delete `spike.rs` once real code uses the crates.
- The design system, tokens, icons, i18n scaffolding and the five screens are ready to be fed
  real data (screens already take data via props).
