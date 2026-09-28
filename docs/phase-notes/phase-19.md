# Phase 19 handoff — iPhone support as an installable web app (PWA)

## Start state / environment

- Branch `v2/p19` (11 commits on top of `v2/p18`). Not merged, not tagged, not
  deployed — awaiting the owner (Standing Rule: no push/merge/tag/deploy unless asked).
- Built on macOS (darwin), Node 25, Rust stable + `wasm32-unknown-unknown`,
  `wasm-pack`, Playwright with the WebKit browser.
- The P19 prompt was pasted; the binding decisions live in
  `docs/00-SYSTEM-CONTEXT.md §18` and the Step-0 research in
  `docs/phase-notes/phase-19-spike.md` (both committed this phase). Read those first.

---

## The one thing to read before continuing (honest status)

**What P19 built is a complete, verified PWA _foundation_ — not yet a functional
staff client end-to-end.** The scaffolding is real and tested: the shared rules and
encryption run in WebKit via WASM; the encrypted local store, PIN, service worker,
manifest, Drive I/O primitives, join primitives, CSP, release build and install/deploy
docs all exist and pass their tests.

**But the browser backend currently serves only two commands** — `create_pin` and
`lock` (`src/lib/web/backend.ts`, `WEB_COMMANDS`). Every other command
(`app_state`, `list_staff`, attendance, notes, check-in, leave, requests, inbox,
collect-fee, **and the Drive sync loop**) still returns `NOT_AVAILABLE_ON_WEB`.
Consequences to be honest about:

- On web today the app boots to a **status screen** (`app_state` isn't served yet), so
  a staff member cannot yet actually use the phone screens in the PWA.
- The Drive route from Step 4 (`src/lib/web/drive/*`) is **built but not wired into a
  screen**, so it is tree-shaken out of the shipped bundle. The `GOOGLE_CLIENT_ID_WEB`
  injection (Step 8) is correct-by-construction (same mechanism as the working
  `VITE_PLATFORM` define) but has no call-site to exercise yet.

So the remaining work to make the PWA a working staff client is the **command
porting + sync wiring** described under "What's left" below. This phase deliberately
laid every hard foundation (WASM parity, crypto, storage, Drive/join primitives, PWA
shell, deploy) so that porting is now mostly plumbing screens to the web backend.

This also still sits behind the **carried P12 live-Drive blocker**: the school-PC
live Drive client (reading `exchange/`, answering join requests, writing acks) has
been deferred since P12 and is unbuilt. A real iPhone end-to-end (join approved by the
school, changes syncing both ways) depends on that being built too.

---

## What this phase did (all committed on `v2/p19`)

### Step 0 — browser Google/Drive spike (`f73cb70`)
Verified against official Google + Apple/WebKit docs (Rule 5): browser-only OAuth for
`drive.file` via GIS token model (no backend/secret); token renewal nuance; Drive REST
v3 CORS; iOS Home-Screen storage exemption from the 7-day ITP cap;
`navigator.storage.persist()` on iOS 17+. Wrote §18 decisions + `phase-19-spike.md`.

### Step 1 — shared core compiled to WASM (`7cfcf5a`, `be0b4f9`, `baf6eb7`)
`crates/vidya-wasm` (wasm-bindgen) re-exports vidya-core decisions, HLC, sealing and
Argon2id across the JS boundary so the PWA runs **byte-identical** rules/encryption.
Sealing + PIN logic were moved *into* vidya-core (pure; caller supplies nonce/salt) so
native (OsRng) and browser (WebCrypto) share one implementation. Native+wasm parity
tests run under `wasm-pack test --node`.

### Steps 1d + 2 — web project + platform + backend choke-point (`891effc`)
`web-pwa/` Vite project reusing `src/` (`@` → `../src`); `platform.ts` gains `'web'`
(isPhone true, isWeb); `src/lib/backend.ts` dispatches `invoke` to Tauri or the Web
backend; `WEB_COMMANDS` map with `NOT_AVAILABLE_ON_WEB` for everything not implemented.

### Step 3 — encrypted local store + PIN + auto-lock (`ab81766`)
IndexedDB record stores + outbox + applied-ops + cursor + attachments + kv; a
**non-extractable** WebCrypto AES-GCM data key; one-transaction writes; PIN
create/unlock/lockout via WASM Argon2id; 5-min-hidden auto-lock.

### Step 4 — Drive route in the browser (`af4d858`)
GIS token client (`drive/auth.ts`), Drive REST v3 (`drive/google.ts`), a `DriveApi`
interface with a `FakeDrive` for tests, and a sync module that seals `.vop` bundles
(WASM) into `exchange/ops-<device>/` and reads acks/peers. **Not yet wired to a screen.**

### Step 5 — join without LAN + migration 0031 (`110a1dc`)
Sealed join request/response through `exchange/joins/`; the invite QR gains the
`/join#d=<payload>` HTTPS form (fragment, host never sees it). Migration `0031`
widened the `device.platform` CHECK to include `'web'` (a real bug — join failed as
"NoSchool" because the web device INSERT was rejected).

### Step 6 — service worker + manifest (`ef63f8d`)
Hand-written SW (no Workbox): precache the shell + hashed JS/CSS/WASM/fonts; Google is
network-only; cache-first same-origin with a cached-index fallback; "new version →
Reload" flow. `manifest.webmanifest` + icons (192/512/maskable), standalone, navy theme.

### Step 7 — iPhone specifics (`0f10ce9`)
`navigator.storage.persist()` after PIN creation; `shareWhatsApp` opens the wa.me deep
link directly on web; `print` goes straight to `window.print()`; `web-pwa/ios.css`
forces ≥16px form fields (no focus-zoom); a new `/teacher/about` **About this iPhone
app** screen (honest limits, en+hi) reached from the account button (web-only). Photos
already use the iOS camera/gallery picker via existing `accept="image/*"` inputs;
Hindi/Telugu fonts already bundled.

### Step 8 — release build + deploy/install docs + CSP (`85f88a6`)
`release.yml` `pwa` job builds the WASM + static site, requires the non-secret
`GOOGLE_CLIENT_ID_WEB` repo Variable (fails clearly if empty), zips
`Vidya-PWA_<ver>.zip`, and `checksums` now hashes it. CSP meta in `index.html`
(only `accounts.google.com` + `www.googleapis.com`, `wasm-unsafe-eval`, inline styles);
`vite.config.ts` injects the client ID + emits `404.html` so `/join` resolves on first
load. New `docs/DEPLOY-PWA.md` (Web OAuth client, hosting on GitHub/Cloudflare Pages,
full CSP + caching headers); `INSTALL.md` gains an iPhone section.

### Step 9 — tests (`8d2d474`)
WebKit e2e (`tests/e2e/pwa/`, `playwright.pwa.config.ts`, `scripts/serve-static.mjs`)
against the production build: boots under the real CSP; **vidya-core rules + sealing +
Argon2id run in WebKit via WASM and match vidya-core exactly**; the SW precaches the
shell (offline-ready); storage.persist + IndexedDB work. Extended the fake-Drive
harness to lock the exchange layout the sync/join route relies on.

---

## What's verified (real output)

- `cargo test --workspace` / `npm run verify` → typecheck + check:hex + check:i18n +
  check:deps + check:version + check:contrast + check:logs + **40 vitest** all green.
- `wasm-pack test --node crates/vidya-wasm` → **5/5** (native + wasm parity: sealing
  round-trip + deterministic, PIN hash/verify/lockout, HLC order, attendance/percent).
- `npx playwright test --config playwright.pwa.config.ts` → **4/4 webkit**.
- `GOOGLE_CLIENT_ID_WEB=… npm run build:web` → `web-pwa/dist/` with valid `sw.js`
  (real version + precache), `404.html` == `index.html`, CSP meta, manifest + icons,
  ~130 KB WASM (wasm-opt disabled in `crates/vidya-wasm/Cargo.toml` — unreliable across
  environments; rustc `opt-level="s"` + strip keep it small).

---

## What's left to make the PWA a working staff client (next phase)

1. **Port the phone-screen commands to `WEB_COMMANDS`** (`src/lib/web/backend.ts`),
   each backed by the encrypted IndexedDB store (Step 3) + WASM rules: `app_state`
   (so the app boots past the status screen), `list_staff`, the attendance flow, notes,
   staff check-in, leave, requests, inbox, and the accountant collect-fee. Each must
   write one transaction (row + audit + op into the outbox) exactly like the Tauri
   backend.
2. **Wire the Drive sync loop** (Step 4) into the app: push the outbox, pull held-
   audience bundles, apply provisionally, surface honest status ("syncs through Google
   Drive"). This is what pulls `web/drive/*` back into the bundle and exercises the
   `GOOGLE_CLIENT_ID_WEB` sign-in.
3. **Wire the join UI** (Step 5 primitives) end-to-end: `/join#d=` → request → poll →
   set PIN → home.
4. **Build the school-PC live Drive client** (the carried P12 blocker): read
   `exchange/`, answer join requests, write acks — required for a real device-to-device
   sync, iPhone included.
5. **On-device verification** the docs can't settle (`phase-19-spike.md` Q5/Q6): real
   iPhone (iOS 16.4+/17+), Add-to-Home-Screen, IndexedDB survives >7 days, persist()
   returns true, camera/gallery picker, tap-to-WhatsApp, print/Save-to-PDF.

---

## Owner action items

- **Confirm the subdomain** (`app.vidya.zuhairhussain.com` is the assumed default, §18).
- **Create the Web OAuth client** (type *Web application*, Authorized JavaScript
  origins = the PWA origin + `http://localhost:5273`, no redirect URIs) in the existing
  Google project, and add the client ID as the repo Variable **`GOOGLE_CLIENT_ID_WEB`**.
  Full steps: `docs/DEPLOY-PWA.md §1–2`.
- **Publish the OAuth consent screen** when ready for all staff (`GOOGLE-OAUTH-SETUP.md`
  Part 5), else only test users can sign in.
- **Host** `web-pwa/dist` on GitHub or Cloudflare Pages with HTTPS + the recommended
  CSP/caching headers (`docs/DEPLOY-PWA.md §4–6`).

---

## Exact commands to re-verify / rebuild

```bash
# full lint + gates + unit tests (green on this branch)
npm run verify

# WASM native + browser parity
wasm-pack test --node crates/vidya-wasm

# build the PWA (WASM + static site) — needs wasm-pack + wasm32 target
GOOGLE_CLIENT_ID_WEB="…apps.googleusercontent.com" npm run build:web

# WebKit PWA tests (installs WebKit once: npx playwright install webkit)
npm run test:e2e:pwa

# preview the built PWA locally
node scripts/serve-static.mjs --port 5280   # then open http://localhost:5280/
```

---

## STOP conditions (prompt) — status

- **New runtime dependency without owner sign-off** → none added. New crates
  (`vidya-wasm`, `wasm-bindgen`, `getrandom[js]`, chacha20poly1305/hmac/argon2 into
  vidya-core) and dev tools (`wasm-pack`, `wasm-bindgen-cli`, WebKit browser) are the
  §18 Step-1 decision + build/test tools; all allow-listed in `scripts/check-deps.mjs`
  (§13). Playwright was already a dependency.
- **Code/docs disagree** → none silently; §18 followed throughout.
- **Invented API/number/copy** → none; hosts, storage policy and OAuth flow verified
  against official docs (`phase-19-spike.md`); attendance % (913) matches vidya-core.
- **Weakened/deleted a test** → none; all suites still pass, new suites added.
- **A real blocker was hit and worked around silently** → NO. The two honest gaps
  (web backend serves only create_pin+lock → the app isn't a functional client yet;
  and the carried P12 live-Drive client is unbuilt) are documented above, not hidden.
