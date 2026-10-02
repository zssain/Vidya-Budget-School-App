# Drive-sync spike — does `drive.file` support the cross-platform exchange? (STOP point)

**Status: BLOCKED on one real-world test the owner must run.** This resolves the
open question carried since `docs/phase-notes/phase-6-spikes.md`. It gates the
multi-device **Drive** sync (desktop ↔ phone) and the **iPhone PWA** sync (Phase D) —
i.e. §18's "iPhone syncs through Google Drive at ₹0". Until it is answered, the Drive
sync client is **not** built (Standing Rule 4 — never assert Google behaviour without
verification).

---

## Why this matters

§18's zero-cost design is: every device reads/writes **sealed `.vop` bundles** in the
school's Google Drive `exchange/` folder, using the minimal **`drive.file`** scope
(non-sensitive → no heavy Google verification). The devices use **different OAuth
clients** by necessity:

- Windows/Mac → a **Desktop** client (`GOOGLE_CLIENT_ID_DESKTOP`)
- iPhone PWA → a **Web** client (`GOOGLE_CLIENT_ID_WEB`)
- Android → an **Android** client (`GOOGLE_CLIENT_ID_ANDROID`)

So for the exchange to work, **one OAuth client must be able to read a file another
OAuth client created** (same Google account, same Cloud project), under `drive.file`.

## What the official docs DO confirm (verified)

Google's scope guide: `drive.file` grants access to *"Create new Drive files, or
modify existing files, that you open with an app or that the user shares with an app
while using the Google Picker API or the app's file picker"* — i.e. **per-file,
per-user, app-created-or-user-picked**. (Sources: Google "Choose Drive API scopes" and
"About authorization" guides, fetched 2026-10-02.)

## What the docs DO NOT settle — the critical unknown (Q-B)

Neither page states whether **"the app"** (for "files created by the app") means the
specific **OAuth client ID** or the whole **Cloud project**. So this is unresolved:

> **Q-B (decisive): With `drive.file`, signed in as the SAME Google account, can Web
> client B read/list a file that Desktop client A created — both in the same "Vidya"
> Cloud project?**

- If **YES** (project-scoped): §18's Drive exchange works across platforms — build it.
- If **NO** (client-scoped): a Web client can only see files *it* created, so the
  iPhone could not read the school PC's bundles (and vice-versa) → **§18's Drive
  exchange does not work as drawn**, and we must switch the iPhone to an alternative
  (see "If Q-B is NO").

Secondary (lower-risk, likely): **Q-A** same client + same account on two devices →
sees its own app-created files (expected YES, standard). **Q-C** different *accounts*
sharing a folder → cross-user read (expected NO — but we avoid this anyway by using
one shared school account).

## The test to run (gets a definitive YES/NO for Q-B)

Uses the real Vidya clients + the school Google account:

1. On the Windows/Mac app, connect Google Drive and run a backup (Phase A,
   `docs/DRIVE-BACKUP.md`) as the **school account** → this creates
   `Vidya/backups/*.vbak` via the **Desktop** client.
2. In a browser, sign in to the **Web** client (`GOOGLE_CLIENT_ID_WEB`) as the **same
   school account** with the `drive.file` scope, and **list** `Vidya/backups/`.
3. **If the web session sees the `.vbak` files → Q-B = YES.** If it sees nothing →
   Q-B = NO.

Because the Web client uses the browser token model (not a redirect), the OAuth
Playground can't stand in for step 2. A **dev-only test harness is now built** that
reuses the real P19 web OAuth (`src/lib/web/drive/auth.ts`), so the result reflects
exactly what the iPhone would see. To run it:

```bash
# one-time: add http://localhost:5273 to the Web client's Authorized JavaScript
# origins, and ensure web-pwa/.env.local has VITE_GOOGLE_CLIENT_ID_WEB.
npm run dev:web
# then open http://localhost:5273/drive-test.html, click the button, and sign in
# as the SAME Google account you used for the desktop backup.
```

The page lists every file this Web client can access and prints a **verdict**: if the
desktop-created `*.vbak` files appear → **Q-B = YES**; if not (and a desktop backup did
run as that account) → **Q-B = NO**. The harness (`web-pwa/drive-test.{html,ts}`) is
**not** part of the production build — it is served only by `dev:web`.

## Design decision tree (what we build after the answer)

- **Q-B = YES →** build one Drive-sync client (push/pull sealed bundles in `exchange/`)
  shared by desktop, Android, and the iPhone PWA. All devices sign in as the **one
  shared school account** (so it only relies on same-account access; sealing +
  audience keys still enforce who can *decrypt* what — Drive is just transport). This
  is the §18 design.
- **Q-B = NO →** the iPhone cannot use the Drive exchange. Fall back to the **relay**
  (P05, already built, normal TLS cert) for iPhone ↔ school-PC sync, keeping Drive for
  **same-client** uses only (e.g. desktop backup). This changes §18's "Drive-only, no
  relay needed" premise for iPhone and should be an explicit owner decision
  (cost/!hosting of the relay).

## STOP

The Drive-sync client and the iPhone sync wiring (Phase D) are **not** started until
Q-B is answered by the test above — guessing here risks building the entire exchange
on a false assumption. The owner-runnable harness is built (`web-pwa/drive-test.html`);
**the next step is for the owner to run it and report YES/NO**, then we build down the
matching branch of the design tree.
