# Android signing — create the release keystore (owner only)

Vidya's Android APKs must be signed with **your own** keystore so you can publish
updates. Vidya's build pipeline never creates or stores this keystore — **you**
create it once, keep it safe, and add it to GitHub as secrets. Until you do,
`release.yml` produces clearly labelled `-UNSIGNED-TEST` (debug-signed) APKs.

> ⚠️ **This file is your responsibility.** The `.jks` keystore and its passwords
> are never committed (they are in `.gitignore`). If you lose them you can never
> update an installed app — see the backup warning at the end.

You need a computer with the **Java JDK** installed (`keytool` comes with it).

---

## a) Create the keystore

Run this once (answer the prompts: your name/org, etc.). Choose a strong
**store password** and **key password** and write them down safely:

```bash
keytool -genkey -v -keystore vidya-release.jks \
  -keyalg RSA -keysize 2048 -validity 10000 -alias vidya
```

This creates `vidya-release.jks` with a key aliased `vidya`, valid ~27 years.

---

## b) Get the SHA-1 fingerprint (for the Google OAuth Android client)

The Android Google-Drive OAuth client (see `docs/GOOGLE-OAUTH-SETUP.md`) needs
this keystore's **SHA-1**:

```bash
keytool -list -v -keystore vidya-release.jks -alias vidya
```

Copy the `SHA1:` line (e.g. `AB:CD:...:EF`). You'll also need the final **package
name** (see `docs/phase-notes/phase-9.md` → "App identifier" — the owner decides
between `in.vidyabudget.app` and `com.zuhairhussain.vidya`).

---

## c) Base64-encode the keystore (for GitHub)

GitHub secrets hold text, so encode the `.jks` to base64:

**macOS / Linux:**
```bash
base64 -i vidya-release.jks | tr -d '\n' > vidya-release.jks.b64
```

**Windows (PowerShell):**
```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes("vidya-release.jks")) | Set-Content -NoNewline vidya-release.jks.b64
```

The file `vidya-release.jks.b64` now holds one long line — its contents go into
the `ANDROID_KEYSTORE_BASE64` secret.

---

## d) Add the GitHub secrets

Repo → **Settings → Secrets and variables → Actions → Secrets → New repository
secret**. Add all four:

| Secret | Value |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | the whole contents of `vidya-release.jks.b64` |
| `ANDROID_KEYSTORE_PASSWORD` | the **store** password from step (a) |
| `ANDROID_KEY_ALIAS` | `vidya` |
| `ANDROID_KEY_PASSWORD` | the **key** password from step (a) |

Once these exist, the next tagged release builds **signed** APKs (no
`-UNSIGNED-TEST` suffix). CI writes them into `keystore.properties` at build time;
that file is gitignored and never committed.

---

## e) ⚠️ Back it up — this is critical

- **Back up `vidya-release.jks` and both passwords in two safe places** (e.g. a
  password manager + an encrypted USB drive kept somewhere else).
- If you **lose the keystore or passwords**, Google/Android will treat a new
  keystore as a *different* app: **every phone with Vidya installed can never
  update** — they'd have to uninstall (losing local data) and reinstall.
- **Never commit** the `.jks`, the `.b64`, or the passwords to git. Delete the
  `.b64` file after pasting it into GitHub.
