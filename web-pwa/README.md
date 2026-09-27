# web-pwa — Vidya iPhone web app (Phase 19)

The installable **staff** web app (PWA). It is the SAME React app as the Tauri build
(shared `src/`, no forked styles — see `@` → `../src` in `vite.config.ts`) compiled
with `platform = 'web'`, so it uses the **Web backend** (IndexedDB + WASM + Google
Drive) and the phone layout. Output is **static files only** — hosted free on GitHub
or Cloudflare Pages at `app.vidya.zuhairhussain.com` (00-context §18). Nothing here
needs a company server.

## Build

| Command | What it does | Output |
|---|---|---|
| `npm run build:wasm` | Compiles `crates/vidya-wasm` for the browser | `web-pwa/wasm/` (gitignored) |
| `npm run build:web`  | `build:wasm` then the Vite build | `web-pwa/dist/` (static site) |
| `npm run dev:web`    | Vite dev server (desktop-browser preview) | — |

## Dependencies (Rule 4 / 00-context §18)

**Runtime** — no new npm runtime dependency; the PWA reuses the app's allowed set
(§13). The only additions are the WASM layer:
- `wasm-bindgen` (Rust crate) — the JS↔WASM binding for `crates/vidya-wasm`. The
  wrapper mechanism named by the P19 Step-1 decision.
- `getrandom` with the `js` feature — the wasm randomness backend (WebCrypto) for the
  §13 rand/argon2/aead crates; gated to `cfg(target_arch = "wasm32")`, never in the
  native app build.

**Build-time tools** (NOT runtime dependencies; installed once, not shipped):
- **`wasm-pack`** — orchestrates the WASM build. Installed via `cargo install
  wasm-pack`; the binary is ~6.3 MB on the build machine.
- **`wasm-bindgen-cli`** — auto-fetched by `wasm-pack` (version pinned to the
  `wasm-bindgen` crate, currently 0.2.x); generates the JS glue + `.d.ts`.

## Measured output (dev/debug build machine, macOS)

- `vidya_wasm_bg.wasm` — **≈ 129 KB** (`wasm-opt`-optimized): all of vidya-core's
  rules + HLC + sealing (ChaCha20-Poly1305/HKDF) + Argon2id.
- `vidya_wasm.js` — ≈ 23 KB glue; typed `vidya_wasm.d.ts` (≈ 7 KB).
- App JS bundle — ≈ 172 KB gzip; fonts (Geist/Newsreader/Devanagari/Telugu subsets)
  lazy-loaded. Well within the ≤ 6 MB compressed PWA target; a final measurement +
  code-splitting pass is Phase 19 Step 8.

The manifest, icons and service worker are added in Step 6; the WASM module is wired
into the Web backend in Step 3.
