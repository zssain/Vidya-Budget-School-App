# PHASE 1 of 10 — FOUNDATION, SIZE SPIKE, DESIGN SYSTEM, PIXEL-EXACT MOCK SCREENS

## ROLE
You are a senior Tauri 2 + React + TypeScript engineer building **Vidya Budget School**.

## STANDING RULES (same in every phase)
1. Read `docs/00-SYSTEM-CONTEXT.md` and `docs/01-MOCK-SPEC.md` IN FULL before anything else,
   then every file in `docs/phase-notes/`.
2. At the start, record: git branch, HEAD commit, `git status`, and tool versions
   (`node -v`, `npm -v`, `rustc -V`, `cargo -V`, JDK, Android SDK/NDK) → top of your handoff.
3. Use only dependencies in context §13, with exactly the listed features. Need anything
   else → STOP and ask (name, purpose, size impact, licence).
4. Never invent features, rules, copy, numbers, API fields, library APIs or config keys.
   For every library call, check the installed crate/package source or its official docs.
   Unsure → STOP and ask.
5. **The mock wins.** Never change a mock colour, size, spacing, font, radius, copy or
   animation. New screens use only existing components and tokens.
6. Business rules only in `crates/vidya-core`. Tauri commands stay thin.
7. No fake success anywhere. No mock backend in release builds.
8. Never delete or weaken a test to make it pass. Never edit a migration committed in an
   earlier phase — add a new one.
9. Run every command you mention; paste real (trimmed) output into the handoff. Report
   environment problems (missing SDK, no device) separately from code failures.
10. Commit in small steps on branch `rebuild/p01`. Do not push, merge, tag or publish
    unless the owner asks.
11. Stop conditions in this prompt are real: stop, write down what you found, ask.
12. Finish with the handoff file (template at the end). A phase is done only when every
    DONE item has evidence.

## OBJECTIVE
Create the project, prove the size budget with every native dependency linked in, build
the design system from the mock, and reproduce the five mock screens so they are
indistinguishable from `design/screens/*.dc.html`, running on desktop and Android. No
database, sync or real logic in this phase.

## DONE MEANS
- `npm run tauri dev` opens on Welcome; `#/__gallery` shows all five screens at mock size.
- Playwright fidelity test passes: every screen and state in 01-MOCK-SPEC §9 differs from
  the mock by ≤ 0.1%.
- Every interaction in 01-MOCK-SPEC §8 works.
- Size spike table filled: download AND installed size per platform with ALL native
  dependencies linked, each ≤ 40 MB / ≤ 50 MB.
- Debug APK installs on a phone/emulator and opens Teacher Home; desktop bundle builds on
  your OS.
- `docs/phase-notes/phase-1.md` written.

## STEPS (do them in this order)

### Step 0 — Repository decision
1. If a git repo with the old Vidya code exists (`src/api/mock`, `crates/vidya-core`):
   - Create branch `rebuild/p01`.
   - `git mv` the old frontend (`src/`, `index.html`, old `vite.config.js`) into `legacy/`.
     Nothing in `legacy/` is built, linted or shipped (exclude it in tsconfig, Vite, CI).
   - Keep `crates/vidya-core` and its tests. Do not change its logic in this phase; only
     make it compile inside the new workspace.
   - Keep `docs/` history; add the new `docs/00-SYSTEM-CONTEXT.md` and
     `docs/01-MOCK-SPEC.md`; move superseded old docs/prompts into `docs/archive/` with a
     one-line note at the top: "Superseded by docs/00-SYSTEM-CONTEXT.md".
2. If there is no repo: `git init`, create the layout in context §14.
3. Copy `design/` from the build pack as-is (screens, assets, brand-kit, reference).
4. Write which path you took in the handoff.

### Step 1 — Toolchain check
Record versions. Required: Node ≥ 20, Rust stable, JDK 17, Android SDK + NDK (install
through Android Studio's SDK Manager if missing; record the NDK version you used — later
phases and CI will pin it). Set `JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME`. If you can't
install Android tooling, STOP and tell the owner exactly what is missing.

### Step 2 — Scaffold
1. Vite React-TS app at the repo root: `npm create vite@latest . -- --template react-ts`
   (or merge into the existing root carefully if files exist).
2. `npm i -D @tauri-apps/cli@^2` then `npx tauri init` with: app name `Vidya`, window
   title `Vidya`, frontendDist `../dist`, devUrl `http://localhost:5173`,
   beforeDevCommand `npm run dev`, beforeBuildCommand `npm run build`.
3. In `src-tauri/tauri.conf.json`: identifier `in.vidyabudget.app`; main window 1440×900,
   minWidth 1280, minHeight 720, centered; bundle targets `nsis`, `dmg`, `app`;
   `bundle.macOS.minimumSystemVersion` `12.0`; `bundle.windows.webviewInstallMode`
   `embedBootstrapper`. Verify every key name against the installed Tauri config schema
   (`src-tauri/gen/schemas` or Tauri docs) — do not guess keys.
4. `npx tauri android init`. Set minSdk 24 in the generated Gradle files.
5. Cargo workspace at the repo root: members `src-tauri`, `crates/vidya-core`.
   Release profile in the root `Cargo.toml`:
   ```toml
   [profile.release]
   lto = true
   codegen-units = 1
   opt-level = "s"
   strip = true
   panic = "abort"
   ```
   (If `panic = "abort"` breaks a Tauri requirement, remove it and note why.)
6. `crates/vidya-core`: if new, an empty lib with one passing test.

### Step 3 — SIZE SPIKE (before any UI work)
Goal: learn the real installer and installed sizes with every native dependency from
context §13 linked in, so nothing surprises us later.
1. Add all Rust dependencies from §13 to `src-tauri/Cargo.toml` with EXACTLY the listed
   features (desktop-only ones under `[target.'cfg(not(target_os = "android"))'.dependencies]`).
2. Create `src-tauri/src/spike.rs` with one `#[tauri::command] fn size_spike()` that
   really uses each crate so the linker keeps it: open an in-memory SQLCipher DB with
   `PRAGMA key` and run `SELECT sqlite_version()`; build a `reqwest::Client`; generate an
   `rcgen` self-signed cert; build a `rustls::ServerConfig` and a `tokio_rustls::TlsAcceptor`;
   build an `axum::Router` with one route; create a `tokio_tungstenite` client request
   (do not connect); create an `mdns_sd::ServiceDaemon` (desktop); read a `keyring` entry
   (desktop, ignore errors); hash with `argon2`; seal with `chacha20poly1305`; verify with
   `ed25519_dalek`; render a QR SVG with `qrcode`. Register the command (never called).
3. Crypto-provider check — both must pass, or STOP:
   - `cargo tree -i aws-lc-rs` → prints nothing (adjust features until it does).
   - `cargo tree -i openssl-sys` → only reachable through `libsqlite3-sys`.
4. Build and measure:
   - Your desktop OS: `npm run tauri build`. Record installer bytes. Installed bytes:
     Windows → run the installer silently into a temp folder (`/S /D=<path>` for NSIS) and
     sum the folder; macOS → `du -sk` of `Vidya.app`.
   - Android: `npx tauri android build --apk --split-per-abi` (release, unsigned is fine
     for measuring). Record each APK's bytes. Set `extractNativeLibs`/legacy packaging so
     native libraries stay inside the APK (check the generated Gradle/Manifest and the
     Android Gradle Plugin docs for the exact setting), enable R8 `minifyEnabled` +
     `shrinkResources` for release, remove unused AndroidX Material dependencies if the
     template added them, and keep only `arm64-v8a` and `armeabi-v7a`.
   - Repeat the Android build once WITH `tauri-plugin-barcode-scanner` and once WITHOUT.
     Record the difference.
5. Write `scripts/size-report.mjs` (plain Node, no deps) that prints a table of artifact
   bytes found under the Tauri bundle output folders and fails (exit 1) if any artifact
   > 40,000,000 bytes. Installed-size checks are added in Phase 9.
6. STOP AND REPORT if any platform exceeds 32 MB download or 42 MB installed (that
   leaves too little headroom), or if the barcode scanner adds more than 2 MB per APK
   (then it is dropped per context §13). Otherwise continue.
7. Keep `spike.rs` for now; Phase 2 deletes it once real code uses the crates.

### Step 4 — Tokens, fonts, keyframes, theme
1. `src/styles/tokens.css`: every token in 01-MOCK-SPEC §2 as CSS variables on `:root`
   (names as in the table), plus radii (4, 6, 8, 10, 16, 18, 20, 22) and the easing
   `--ease: cubic-bezier(.2,.8,.2,1)`.
2. `src/styles/app.css`: `@import "tailwindcss";` then `@theme` mapping tokens so classes
   like `bg-navy`, `text-muted`, `border-line`, `bg-accent`, `bg-panel`, `rounded-card`
   exist. Arbitrary values (`text-[13px]`, `tracking-[0.16em]`) are fine where the mock
   uses one-off values.
3. `src/styles/keyframes.css`: every keyframe in 01-MOCK-SPEC §5 with the app names
   (`vRise12`, `vRise14`, `vIn6`, `vIn8`, `vIn10`, `vGrow`, `vFill`, `vSheet`, `vFade`,
   `vPopFee`, `vPopAtt`, `vPulse`), the global button transition, the three `:active`
   scales, and the reduced-motion rule.
4. Fonts: install the Fontsource packages (verify exact names on npm): Geist
   400/500/600/700 Latin; Newsreader VARIABLE with the `opsz` axis (normal + italic),
   Latin; Noto Sans Devanagari 400/500/600, Devanagari + Latin. Import only those subset
   files in `src/main.tsx`. `grep -r "fonts.googleapis" src/ index.html` must print
   nothing.
5. `src/lib/theme.ts`: `setAccent(hex)` implementing the helper in 01-MOCK-SPEC §2.3;
   default `#2F7479`; persists to `localStorage` for now (Phase 3 moves it to settings).

### Step 5 — Mock renderer (dev only) for fidelity testing
Write `design/runtime/support.js` (plain JS, no deps, ~200 lines) so the unmodified mock
files render in a browser:
- Each mock's `<head>` loads `./support.js`; serve `design/` in DEV ONLY at `/design/`
  (Vite `server.fs.allow` + a dev-only middleware that serves `design/runtime/support.js`
  for `/design/screens/support.js`). Never include `design/` in `dist/`.
- Implement: parse `<x-dc>` template once; find the `<script type="text/x-dc">`, evaluate
  its class with a provided `DCLogic` base (`props` from `data-props` defaults, `state`,
  `setState` → re-render, `componentWillUnmount` optional); resolve `{{a.b}}` holes in
  text and attributes; `<sc-for list as>` with `$index`; `<sc-if value>`; `onClick` /
  `onChange` / `onInput` bound to returned functions; boolean attributes (`disabled`,
  `aria-*`) from values; move `<helmet>` contents into `<head>`; rewrite `/_blob/<id>`
  to `../assets/<file>` using the table in 01-MOCK-SPEC §4. Re-render by rebuilding the
  root DOM from the template (simple is fine) while keeping focus in inputs.
- Add a dev page `#/__mocks` in the app listing links to the five mock files.

### Step 6 — Icons and primitives
1. `src/lib/icons.ts`: copy every SVG path from the mocks into named icons (list in
   01-MOCK-SPEC §6). `src/components/Icon.tsx` renders `<svg>` with `size`, `strokeWidth`
   (default 1.6), `color`, and optional two-tone parts for tiles.
2. shadcn/ui: `npx shadcn@latest init` for Tailwind v4 (verify the current init
   options in shadcn's docs); add only `button input dialog dropdown-menu select tabs
   radio-group checkbox`. Delete every `lucide-react` import and the dependency; replace
   icons with `Icon`. Restyle each primitive to the mock values (radius 6 inputs/buttons,
   borders `#C9D3D2`/`#D5DDE0`, focus ring `0 0 0 3px var(--accent-12)` on EVERY
   interactive element, no default shadows).
3. `npm ls lucide-react` must report nothing.

### Step 7 — Components
Build every component in 01-MOCK-SPEC §7 in `src/components/`, props-only (no sample data
inside). For each: a gallery entry in `src/dev/Gallery.tsx` showing all variants. Match
the listed values; when in doubt open the mock and copy the inline style.

### Step 8 — Platform, router, layouts
1. Platform is decided at BUILD time: in `vite.config.ts` read
   `process.env.TAURI_ENV_PLATFORM` (set by the Tauri CLI during `tauri dev/build` and
   `tauri android dev/build` — verify in Tauri docs) and `define`
   `import.meta.env.VITE_PLATFORM` = `android` | `desktop`. In a plain browser
   (`npm run dev`) allow `?platform=android` override, DEV only.
   `src/lib/platform.ts` exports `isPhone`.
2. `src/lib/router.ts`: hand-written hash router with `useSyncExternalStore`,
   `navigate(path)`, params (`/teacher/attendance/:classId`).
3. Routes: `#/welcome` (default), `#/principal/home`, `#/accountant/collect`,
   `#/teacher/home`, `#/teacher/attendance/:classId`, `#/__gallery`, `#/__mocks` (both
   DEV only, compiled out of release: guard with `import.meta.env.DEV`). Every other
   sidebar link and tile → `Placeholder` screen.
4. Layouts: `DesktopShell` (Sidebar + Header + scrollable main), `PhoneShell` (safe areas
   via `env(safe-area-inset-*)`).

### Step 9 — i18n and formatting
1. `src/lib/i18n/index.ts`: `t(key, vars)`; `en.json` holds EVERY visible string from the
   mocks, word for word; `hi.json` has identical keys with values `"TODO-HI: <english>"`.
   No hard-coded user-visible strings in components or screens.
2. `src/lib/format.ts` + Vitest tests: `formatMoney(paise)` (Intl `en-IN`, "₹" prefix, no
   paise if zero → "₹6,84,200"), `formatDateLong` ("Wednesday, 23 September"),
   `formatSession` ("2026–27"), `formatRelative` ("12 s ago", "2 min ago", "5 hours ago",
   "1 day ago", "2 days ago"), `greeting` (morning < 12, afternoon < 17, else evening),
   `parseDigits(str, maxLen)`.

### Step 10 — The five screens with fixtures
1. `src/dev/fixtures/{welcome,principalHome,collectFee,teacherHome,attendance}.ts`: copy
   the sample data VERBATIM from each mock's script block (names, amounts, 34 students,
   initial marks, approvals, classes, fee days).
2. Build each screen from components, matching the mock exactly, including every
   interaction in 01-MOCK-SPEC §8 and every animation in §5.1. Screens receive data via
   props from a fixture provider in this phase (Phase 3 swaps in real data).
3. Welcome on phone: single column, navy panel hidden, same components.
4. Edge cases: window 1280×720 (no overlap; main column scrolls; ellipsis only where the
   mock uses it); phone widths 360–430 (tiles stay 3 columns, targets ≥ 44px); Android
   safe areas; a Hindi test string renders in Noto Sans Devanagari; keyboard focus ring
   everywhere; reduced motion.

### Step 11 — App icons
Copy `design/brand-kit/windows/vidya.ico`, `macos/vidya.icns`, `png/*` into
`src-tauri/icons/` with the names `tauri.conf.json` expects; copy
`design/brand-kit/android/res/*` into the generated Android `res/` (adaptive icon with
background `#0C1B38`). Do NOT run an icon generator.

### Step 12 — Fidelity tests
`tests/e2e/fidelity.spec.ts` (Playwright): for each screen/state in 01-MOCK-SPEC §9, open
the mock through the renderer and the app gallery route at the same viewport (1440×960,
1440×1080, 390×844), wait 1.5 s, and compare with `toHaveScreenshot` (max diff ratio
0.001). First run generates the baseline FROM THE MOCK, then asserts the app against it.
Also `tests/e2e/phase1.spec.ts`:
- Collect fee: type 5000 → error text + disabled button; Full due chip → balance ₹0;
  Cash hides the reference field.
- Attendance: submit disabled with "4 students not marked yet"; Mark all → enabled; Undo
  restores; Submit locks the rows and shows the submitted bar.
- Welcome: each radio swaps field and CTA.

### Step 13 — Builds
- `npm run tauri build` on your OS succeeds.
- `npx tauri android build --apk --debug` succeeds; install on an emulator or phone; the
  app opens in phone layout (platform is set at build time, so `?platform` is not needed
  there). For this phase only, a DEV-only flag `VITE_DEV_START=teacher` makes it open on
  Teacher Home; Phase 3 removes the flag.
- Run `scripts/size-report.mjs`.

## VERIFICATION (all must pass)
- `npx tsc --noEmit` clean; `npx vitest run` green; `cargo test --workspace` green.
- Playwright fidelity + phase1 specs green; screenshots saved to `tests/e2e/__screens__/`.
- `grep -r "fonts.googleapis" src index.html` → nothing; `npm ls lucide-react` → nothing;
  no animation library in `package.json`.
- `cargo tree -i aws-lc-rs` → nothing.
- Size table complete.

## STOP CONDITIONS
- Size limits from Step 3.6 exceeded.
- aws-lc-rs cannot be removed.
- A mock detail can't be reproduced (e.g. font rendering difference > threshold that you
  cannot fix) → report with screenshots, don't "approximate".
- Android tooling unavailable.

## HANDOFF → `docs/phase-notes/phase-1.md`
Sections: Start state · Repo path taken (existing vs fresh, what moved to legacy/) · What
was built (components with props, routes, files) · Commands + real output · Test results ·
**Size spike table** (platform, download bytes, installed bytes, barcode-scanner delta,
top 5 biggest crates from `cargo bloat` if available as a tool) · Fidelity results per
screen/state (diff %) · Deviations (should be none) · [OWNER] defaults used · Known issues
· Questions · What Phase 2 needs.
