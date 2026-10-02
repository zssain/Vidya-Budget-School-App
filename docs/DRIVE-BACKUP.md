# Desktop Google Drive backup — setup & verification

Vidya's Windows/Mac app keeps an **encrypted local backup** on the PC automatically.
Phase A ("#1") adds an **off-site copy in Google Drive** so a lost, stolen, or broken
PC never loses the school's records. The Drive copy uses the minimal **`drive.file`**
scope — Vidya can see only the files it itself creates (a `Vidya/backups/` folder),
never the rest of your Drive.

Local backup **always** runs. Drive is an *additional* copy: when connected, every
backup is also uploaded and verified, and old Drive copies are pruned on the same
30-daily / 12-monthly schedule as local. If Drive is down or disconnected, the backup
still succeeds locally (status "Backed up · this PC") — a Drive problem never fails a
backup.

---

## 1. Prerequisite — the Desktop OAuth client id

You need the **Desktop** OAuth client id (`GOOGLE_CLIENT_ID_DESKTOP`) from
`docs/GOOGLE-OAUTH-SETUP.md` Part 6. No client secret is needed (the desktop flow uses
PKCE). Make it available to the build:

- **Release build (CI):** add the repo **Variable** `GOOGLE_CLIENT_ID_DESKTOP`
  (Settings → Secrets and variables → Actions → Variables). `write-release-config.sh`
  bakes it into the build.
- **Local test build:** put it in `src-tauri/build-config/dev.json`:
  ```json
  { "google_client_id_desktop": "…apps.googleusercontent.com", "google_client_id_android": "" }
  ```

While your OAuth consent screen is in **Testing**, the Google account you sign in with
must be on the **Test users** list (or publish the app). For backup, that is the
account whose Drive will hold the backups.

---

## 2. Verify the live sign-in + upload (owner, ~5 min)

> This is the one part that **cannot** be tested in CI — it needs an interactive Google
> consent in a real browser. Everything else (the OAuth/PKCE logic, the Drive REST
> client, the local + fake-Drive backup path) is covered by automated tests.

On a desktop build with the client id configured (step 1):

1. Open Vidya, set up or open a school, and **turn on backups** (enter the recovery
   key) on the **Backups** screen.
2. In the **Google Drive (off-site copy)** card, click **Connect Google Drive**. Your
   browser opens Google's sign-in.
3. Sign in with the backup account and **allow** the `drive.file` permission. The tab
   shows "Vidya is connected to Google Drive — you can close this tab." The card now
   shows **Connected**.
4. Click **Back up now**. After it finishes, the history row reads **"Backed up ·
   this PC and Drive"**.
5. In that Google account's Drive, confirm a file exists at
   **`Vidya / backups / vidya-<school>-<date>-<time>.vbak`**.
6. (Optional) Click **Disconnect** — the card returns to "Connect Google Drive" and
   backups continue local-only.

If sign-in shows an `invalid_client` / origin error, the client id in the build does
not match the Desktop client (step 1). If it says access is blocked / app not verified,
add the account under **Test users** (or publish the consent screen).

---

## 3. Restoring a school on another PC

A backup is only useful if you can restore it, so test the round-trip once:

1. On a **fresh PC** (or after reinstalling), open Vidya → on the Welcome screen pick
   **Recover an existing school** → **Continue**. You land on the Recover screen.
2. Choose the backup, either way:
   - **Restore from Google Drive** — sign in as the backup account; the newest
     backups are listed; click **Use this**. (This is the normal path — a `.vbak`
     downloaded from Drive's website on its own **cannot** be restored, because its
     salt lives in the file's Drive properties, which a plain download does not carry.)
   - **Choose a backup file (.vbak)** — only works when the file's **`.vbak.meta`**
     sidecar is beside it (e.g. copied from another PC's `…/Vidya/backups/` folder).
3. Enter the **recovery key** → **Check this backup** → confirm the school name, date
   and counts look right → **Restore this school**.
4. Vidya installs it and **restarts** into the restored school. The previous PC is
   fenced off (its devices must rejoin); your prior data on this PC (if any) is kept
   as a `vidya.db.pre-restore-…` safety copy.

> The install→restart and the live Drive sign-in can only be confirmed on a real
> build; everything underneath (key derivation, staged atomic install, fencing) is
> covered by tests.

---

## 4. What's automated vs manual

| Piece | Covered by |
|---|---|
| PKCE (S256), auth-URL, loopback redirect parsing, token freshness | `sync::drive::oauth` unit tests |
| Drive REST error mapping, JSON→file mapping, multipart upload body, folder query | `sync::drive::google` unit tests |
| Backup upload + verify-by-readback + prune, outcome (Verified/Partial) | `backup` tests with a fake Drive |
| "Not connected → local-only, never fails" | `backup::schedule` test |
| Salt carried to `.meta` + Drive appProperties | `backup` test |
| Restore: key from recovery+salt, summary, staged atomic install + fencing | `backup::restore` tests |
| **Live Google sign-in + real upload** | **manual (section 2)** |
| **Restore → app restarts into the recovered school** | **manual (section 3)** |
