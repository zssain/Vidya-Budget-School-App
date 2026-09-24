# Vidya v1.0.0 — owner release checklist

One ordered list. Do the steps top to bottom; each links to the guide with the
detail. Steps 1–6 are one-time setup; after that, cutting a release is steps 7–11.
Nothing here is automatic — tagging is the only thing that starts a build.

> Signing note: **Windows and macOS ship UNSIGNED for v1.0.0** (your decision).
> No `WINDOWS_*` / `APPLE_*` secrets are needed. Android is signed with **your**
> keystore (steps 2–3); without it, Android APKs are labelled `-UNSIGNED-TEST`.

---

### One-time setup

- [ ] **1. Confirm the app identifier.** It can NEVER change after the first
  public release. Current value: `in.vidyabudget.app`. You are considering
  `com.zuhairhussain.vidya`. Decide now — see `docs/phase-notes/phase-9.md` →
  *App identifier* for the trade-offs. The Google Android OAuth client (step 6)
  must use whichever you pick.

- [ ] **2. Create the Android release keystore and BACK IT UP** in two safe
  places. → `docs/ANDROID-SIGNING.md` (steps a–b, e). Keep the SHA-1 for step 6.

- [ ] **3. Add the four Android secrets** to GitHub (Settings → Secrets and
  variables → Actions → **Secrets**): `ANDROID_KEYSTORE_BASE64`,
  `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS=vidya`, `ANDROID_KEY_PASSWORD`.
  → `docs/ANDROID-SIGNING.md` (steps c–d).

- [ ] **4. Deploy the licence + relay services on Fly.io and add DNS** so
  `api.vidya.zuhairhussain.com` and `relay.vidya.zuhairhussain.com` work.
  → `docs/DEPLOY-FLY.md`.

- [ ] **5. Generate the production licence key.** Run
  `cd cloud/licence && cargo run -- gen-prod-key`. Set the PRIVATE key as the
  licence server's `LICENCE_SIGNING_KEY` Fly secret; add the PUBLIC key as the
  GitHub **Variable** `LICENCE_PUBLIC_KEY`. Never commit either.
  → `docs/DEPLOY-FLY.md`.

- [ ] **6. Create the two Google OAuth clients** (Desktop + Android, Android
  needs the step-1 package name + step-2 SHA-1). Add their IDs as GitHub
  **Variables** `GOOGLE_CLIENT_ID_DESKTOP` and `GOOGLE_CLIENT_ID_ANDROID`.
  → `docs/GOOGLE-OAUTH-SETUP.md`.

  Also add the two host **Variables**: `LICENCE_API=https://api.vidya.zuhairhussain.com`
  and `RELAY_URL=wss://relay.vidya.zuhairhussain.com`. (These five Variables are
  the build-config; a release fails clearly if any is missing.)

### Cut the release

- [ ] **7. Merge the release branch to `main`** (or push the branch you tag).

- [ ] **8. Tag and push:** `git tag v1.0.0 && git push --tags`. This starts
  `.github/workflows/release.yml`.

- [ ] **9. Review the DRAFT GitHub Release.** Check: all files present
  (`Vidya_1.0.0_x64-setup.exe`, `Vidya_1.0.0_universal.dmg`,
  `Vidya_1.0.0_arm64-v8a.apk`, `Vidya_1.0.0_armeabi-v7a.apk`, `SHA256SUMS.txt`);
  each download ≤ 40 MB; the notes carry the "Unsigned build" banner; APKs are
  NOT `-UNSIGNED-TEST` (i.e. signing worked).

- [ ] **10. Clean-machine runs 1–6** — install on real devices and record
  date/device/result. → `docs/phase-notes/phase-9.md` → *Clean-machine runs*.

- [ ] **11. Publish** the release (un-draft it) once 9 and 10 pass.

---

### The five build-config Variables (GitHub → Actions → Variables)

| Variable | Value | Source |
|---|---|---|
| `LICENCE_API` | `https://api.vidya.zuhairhussain.com` | your Fly licence app (step 4) |
| `RELAY_URL` | `wss://relay.vidya.zuhairhussain.com` | your Fly relay app (step 4) |
| `LICENCE_PUBLIC_KEY` | base64 ed25519 public key | `gen-prod-key` (step 5) |
| `GOOGLE_CLIENT_ID_DESKTOP` | desktop OAuth client id | Google console (step 6) |
| `GOOGLE_CLIENT_ID_ANDROID` | Android OAuth client id | Google console (step 6) |

These are **Variables** (not secrets) — they are public values. The only true
secrets are the Android keystore (step 3) and the licence PRIVATE key, which
lives on the Fly server, never in the app build.
