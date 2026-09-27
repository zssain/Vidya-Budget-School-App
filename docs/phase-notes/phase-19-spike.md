# Phase 19 — Step 0 spike: Google sign-in & Drive from a browser (PWA)

**Verdict: PROCEED (no hard STOP).** A browser-only PWA can use Google Drive (`drive.file`)
with **no backend and no client secret**; Drive REST v3 accepts browser (CORS) requests;
iOS Home-Screen web apps are **exempt from WebKit's 7-day storage cap**. The one caveat is
token renewal (Q2): the browser has **no refresh token**, so renewal is silent *while the
Google session is active* but needs an occasional **re-sign-in tap** when it lapses — usable,
not impossible, so the Step-0 STOP ("silent renewal is impossible") does **not** fire. The
owner should be aware of the re-tap UX, and four items need confirmation on a real iPhone
(§Real-device).

This spike was verified against Google's and Apple/WebKit's **official documentation** (Rule 5).
The live on-device confirmations and the OAuth-client creation are the owner's to perform.

## Q1 — Browser-only OAuth for `drive.file` — CONFIRMED
Use the **Google Identity Services (GIS) token model** (OAuth 2.0 *implicit* grant):
`google.accounts.oauth2.initTokenClient({client_id, scope, callback})` then
`tokenClient.requestAccessToken(...)`. Needs only a **Web application** OAuth client ID with
**Authorized JavaScript origins** — **no client secret, no server**. Returns a short-lived
(**1 hour**, `expires_in: 3600`) access token in the browser. Google's own Drive **JS
quickstart** is browser-only and calls the Drive REST API directly with this flow.
- The PWA (a **staff client**) needs only **`drive.file`** for sync. It does **not** need
  `gmail.send` — email is sent by the school PC (00-context §10.1), never by a device.
- Docs: identity/oauth2/web/guides/use-token-model · .../how-user-authz-works ·
  identity/protocols/oauth2/javascript-implicit-flow · workspace/drive/api/quickstart/js

## Q2 — Token renewal without a backend — CONDITIONAL (the key nuance)
There is **no refresh token** in the browser token model, and GIS **removed** automatic
access-token refresh. A fresh 1-hour token is obtained by calling `requestAccessToken` with
**`prompt: ''`** (or `'none'`) — this returns a token **without a consent/account screen** when
the user has an **active Google session AND prior consent** (consent is remembered per user +
client ID). When the session/consent has lapsed, GIS shows UI (a tap). GIS also notes a user
gesture is generally required to invoke `requestAccessToken()` in implicit mode.
- **Consequence for Vidya:** renewal is effectively silent during normal use, but is **not**
  guaranteed unattended background refresh. The app must handle a lapsed token by asking the
  signed-in staff member to **tap "Reconnect Google Drive"** (a short re-consent). Status copy
  must be honest: "Tap to reconnect Vidya to Google Drive" rather than a fake "synced".
- This does **not** meet the STOP bar ("silent renewal is impossible") — it is possible while
  the session is alive. Owner is informed of the re-tap fallback.
- Docs: identity/oauth2/web/guides/use-token-model · web/reference/js-reference
  (`prompt`/`OverridableTokenClientConfig`) · web/guides/migration-to-gis (auto-refresh removed)

## Q3 — Drive REST v3 from the browser (CORS) — CONFIRMED
Drive REST v3 answers cross-origin browser calls for **`files.list`**, **`files.get?alt=media`**
(download) and **multipart upload** (`POST /upload/drive/v3/files?uploadType=multipart`, ≤ 5 MB
in one request — Google documents a browser `fetch()` + `Blob` multipart sample). Google's
browser-only JS quickstart calling `www.googleapis.com` directly is the load-bearing proof.
- Docs: workspace/drive/api/quickstart/js · workspace/drive/api/guides/manage-uploads ·
  drive/api/reference/rest/v3/files/{list,get} · drive/api/v3/savetodrive (Drive CORS guidance)

## Q4 — `gmail.send` classification — CONFIRMED (not the PWA's concern)
`https://www.googleapis.com/auth/gmail.send` is a Google **Sensitive** scope (not Restricted):
production use needs **Sensitive-scope app verification** (~3–5 business days), **not** the
Restricted-scope security audit. Only the **school PC** uses it; the PWA does not.
- Docs: workspace/gmail/api/auth/scopes · identity/.../sensitive-scope-verification

## Q5 — iOS Home-Screen storage retention — CONFIRMED (resolves the §18 [VERIFY])
Per WebKit:
- A Home-Screen web app has its **own storage/cookie container, isolated from Safari**.
- The **first-party domain of Home-Screen web apps is EXEMPT from ITP's 7-day cap** on
  script-writable storage (IndexedDB, LocalStorage, SessionStorage, Media keys, Service Worker
  registrations + cache). "We do not expect the first-party in such a web application to have its
  website data deleted"; WebKit treats such deletion as "a serious bug."
- **Separate caveat:** WebKit's general low-disk / LRU eviction is distinct from the ITP cap and
  is not time-guaranteed — under severe storage pressure a non-persistent origin can still be
  evicted. Hence `navigator.storage.persist()` (Q6).
- **Honest-limits copy update (Step 7 About screen):** replace "Safari can clear website data if
  unused" with the accurate "Installed to your Home Screen, Vidya's data is kept (it is exempt
  from Safari's 7-day cleanup); only very low iPhone storage could clear it — keep Vidya synced."
- Docs: webkit.org/blog/10218 · webkit.org/tracking-prevention · webkit.org/blog/14403

## Q6 — `navigator.storage.persist()` on iOS — CONFIRMED (iOS 17+)
Supported since **Safari 17 / iOS 17**; persistent-mode origins are **excluded from eviction**,
and WebKit **grants persistence heuristically — including when the site is a Home-Screen web
app**. On **iOS ≤ 16 the API is effectively absent** — do not rely on it there. So P19 requests
`navigator.storage.persist()` after PIN creation (§decisions) and treats a `false` result as
"best-effort storage" honestly.
- Docs: webkit.org/blog/14403 (Updates to Storage Policy)

## OAuth client to create (owner — Google Cloud console, existing project)
Add a **"Web application"** OAuth 2.0 client:
- **Authorized JavaScript origins:** `https://app.vidya.zuhairhussain.com` (the PWA domain,
  [OWNER confirms subdomain]) and, for local testing, `http://localhost:5173`.
- **Scope requested at runtime:** `https://www.googleapis.com/auth/drive.file` only.
- No redirect URI / no client secret is needed for the token model (the client secret Google
  also generates is for server-side use only — never shipped).
- Add the resulting client ID as a build-config value `google_client_id_web` (new key alongside
  the desktop/android ones) → GitHub Variable `GOOGLE_CLIENT_ID_WEB`. (Wired in Step 1/8.)

## Real-device verification the owner must do (docs can't settle these)
On a real iPhone at the target iOS version, with the app installed to the Home Screen:
1. IndexedDB / service-worker data **survives > 7 days of no use** (ITP exemption holds in
   practice).
2. `navigator.storage.persist()` **returns `true`** for the installed app and `persisted()`
   stays true across relaunches (iOS 17+; ≤16 lacks it).
3. The **Google session/consent survives** in the isolated container so
   `requestAccessToken({prompt:''})` returns a token without UI across app relaunches — and how
   often the re-sign-in tap is actually needed.
4. Behavior under **low storage** and after **iOS updates**.

## STOP conditions (phase) — status
- "Google sign-in can't be kept usable without a backend" — **not hit**: usable via GIS token
  model with a re-tap fallback when the session lapses (Q1/Q2).
- "Any need for a company server" — **not hit**: static host + the school's own Google Drive.
- "Apple rules that block a required feature" — **none found** for the staff-client scope; the
  known limits (no background sync, web push out of scope, camera/gallery picker) are the
  documented §18 honest limits, not blockers.

## The larger prerequisite (carried from P12/P18, unchanged)
The PWA syncs and **joins** through the sync account's Drive, and joining requires the **school
PC** to process a sealed join request from Drive and answer it (Step 5). That PC-side **live
Drive client was deferred since P12 and is not yet built** (needs the owner's real Google
accounts). P19 can build and unit-test the PWA's Drive/join code against the **fake Drive**
(as P12/P14/P15 did), but the end-to-end DONE-MEANS (real iPhone joins a real school over real
Drive) cannot be exercised until the P12 live-Drive work is completed on the owner's machine.
This is the same blocker flagged throughout P18; it is the true gate for both a multi-device
v2.0.0 and this phase's on-device acceptance.
