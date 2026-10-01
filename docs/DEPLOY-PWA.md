# Deploying the Vidya iPhone web app (PWA)

Phase 19 ships iPhone support as an **installable web app (PWA)** so staff can add
Vidya from Safari → **Share → Add to Home Screen** at **₹0** — no Apple Developer
account, no App Store. This document is how you (the owner) put that web app online.

It is a **staff client only** (Teacher/Accountant phone screens). It never runs the
school server, setup, restore, backups or licences — those are Windows/macOS only.

---

## 0. What you are deploying

`npm run build:web` produces a folder of **static files** in `web-pwa/dist/`:

- `index.html`, hashed JS/CSS, the WebAssembly module (`vidya-core` rules + sealing
  + Argon2id, identical to the app), bundled Hindi/Telugu/Latin fonts;
- `sw.js` — the service worker (offline app shell + "new version → Reload");
- `manifest.webmanifest` + the app icons;
- `404.html` — a copy of `index.html` so the `/join` deep-link works on first load.

These are the whole app. The host only ever serves files — **it never sees any
school data**. All data is sealed in the browser and syncs through the school's
Google Drive (§18). Any static host with **HTTPS** works; the two free options below
cost nothing.

The release workflow also builds this bundle on every tag and attaches it as
`Vidya-PWA_<version>.zip` (see `.github/workflows/release.yml`).

---

## 1. Prerequisite — a **Web** OAuth client ID

The desktop and Android apps use their own OAuth client IDs (see
`docs/GOOGLE-OAUTH-SETUP.md`). The PWA needs a **third** client, of type
**Web application**, in the **same** Google Cloud project and the **same** consent
screen (same `drive.file` scope — Vidya only ever touches files it created).

1. Google Cloud Console → **APIs & Services → Credentials → Create credentials →
   OAuth client ID**.
2. **Application type: Web application.** Name it `Vidya Web (PWA)`.
3. Under **Authorized JavaScript origins**, add the exact origin(s) the app is served
   from — scheme + host, **no path, no trailing slash**:
   - `https://app.neverworks.org` — the production origin **[OWNER confirms
     the subdomain]**;
   - `http://localhost:5273` — only if you want to test the dev server locally.
4. **Leave "Authorized redirect URIs" empty.** The PWA uses the Google Identity
   Services **browser token model** (`initTokenClient` / `requestAccessToken`), which
   authorizes by origin and returns the token in-page — there is no redirect URI and
   no client secret.
5. Click **Create** and copy the **Client ID** (it ends in
   `.apps.googleusercontent.com`). This is a **non-secret** build value, exactly like
   the desktop/Android IDs.

> If your consent screen is still in **Testing**, only the emails you added as test
> users can sign in (`docs/GOOGLE-OAUTH-SETUP.md` Part 5). Publish it when you are
> ready for all staff.

---

## 2. Give the build the client ID

**In CI (recommended):** add a repo **Variable** (not a secret — it is public):

- GitHub → **Settings → Secrets and variables → Actions → Variables → New variable**
- Name: `GOOGLE_CLIENT_ID_WEB` · Value: the Web client ID from step 1.

The `pwa` job in `release.yml` reads it and **fails the build if it is empty** (no
fake sign-in). Add it before you cut the first PWA release.

**Locally:** either export it in your shell before building —

```bash
GOOGLE_CLIENT_ID_WEB="…apps.googleusercontent.com" npm run build:web
```

— or create `web-pwa/.env.local` with `VITE_GOOGLE_CLIENT_ID_WEB=…` (git-ignored).
If it is missing, the app still builds and runs but shows a "sign-in not set up yet"
state instead of pretending to connect.

---

## 3. Build and test locally first

```bash
npm run build:web            # builds the WASM, then the static site into web-pwa/dist/
npx http-server web-pwa/dist -p 8080   # or any static server
# then open http://localhost:8080/
```

Sign-in to Google only works from an **Authorized JavaScript origin** (step 1), so
full Drive testing needs the real HTTPS origin or `http://localhost:5273` via
`npm run dev:web`.

---

## 4. Option A — GitHub Pages

1. Put `web-pwa/dist/` at the site root of a Pages-served branch/repo (e.g. copy it
   into a `docs/` or `gh-pages` publish path, or use a Pages deploy action gated on
   the release). Because everything is relative to `/`, deploy it at the **domain
   root**, not a sub-path.
2. GitHub → **Settings → Pages** → choose the branch/folder.
3. Add a `CNAME` file containing `app.neverworks.org` **[OWNER confirms]** so
   Pages serves the custom domain over HTTPS.
4. The emitted `404.html` makes the `/join` deep-link resolve to the app on first
   load — no extra config needed.

## 5. Option B — Cloudflare Pages

1. Cloudflare Dashboard → **Workers & Pages → Create → Pages → Connect to Git** (or
   **Direct Upload** of `web-pwa/dist/`).
2. If building from Git: **Build command** `npm run build:web`, **Output directory**
   `web-pwa/dist`, and add the environment variable `GOOGLE_CLIENT_ID_WEB`.
3. Add a `_redirects` file (or commit one into `web-pwa/public/`) so paths fall back
   to the app:

   ```
   /*  /index.html  200
   ```
4. Add `app.neverworks.org` under **Custom domains**.

Both hosts give free automatic HTTPS, which iOS **requires** for a Home-Screen web
app, for the service worker, and for Google sign-in.

---

## 6. Recommended HTTP response headers

The app ships a `Content-Security-Policy` **meta tag** in `index.html` as a baseline.
For defence-in-depth, configure your host to send the **header** form too — it is
strictly enforced and can use directives a meta tag ignores (`frame-ancestors`):

```
Content-Security-Policy: default-src 'self'; base-uri 'self'; object-src 'none'; img-src 'self' data: blob:; font-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self' 'wasm-unsafe-eval' https://accounts.google.com; connect-src 'self' https://www.googleapis.com https://accounts.google.com https://*.googleusercontent.com; frame-src https://accounts.google.com; worker-src 'self'; manifest-src 'self'; form-action 'self'; frame-ancestors 'none'
Referrer-Policy: strict-origin-when-cross-origin
X-Content-Type-Options: nosniff
```

The PWA contacts exactly two external hosts — `accounts.google.com` (the GIS sign-in
library + token flow) and `www.googleapis.com` (Drive REST) — plus
`*.googleusercontent.com` for Drive media redirects. `'wasm-unsafe-eval'` is what lets
Safari 16.4+ instantiate the WebAssembly module; `'unsafe-inline'` **style** is needed
because the shared React screens use inline `style=` attributes (this does **not**
allow inline scripts).

### Caching

- `sw.js`, `index.html`, `404.html`, `manifest.webmanifest` → **no long cache**
  (`Cache-Control: no-cache`), so a new release is picked up promptly.
- `/assets/*` (content-hashed filenames) → **immutable, long cache**
  (`Cache-Control: public, max-age=31536000, immutable`).

The service worker already versions its own cache by content hash, so a stale CDN is
the only thing to guard against here.

---

## 7. After deploying

1. On an iPhone (iOS 16.4+), open the URL in **Safari** → **Share → Add to Home
   Screen** (see `INSTALL.md`).
2. Open Vidya from the **Home Screen** (not the Safari tab) so it runs standalone and
   its storage is retained.
3. Join the school with the invitation QR/link, set a PIN, and confirm a change syncs
   to another device within ~1–2 minutes through Drive.

If sign-in fails with a `redirect_uri`/origin error, the origin you are serving from
is not in the client's **Authorized JavaScript origins** (step 1) — add it exactly,
including `https://` and no trailing slash.
