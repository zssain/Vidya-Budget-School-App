# Vidya v2.0.0 — owner release checklist

One ordered list. Do the steps top to bottom; each links to the guide with the
detail. Steps 1–7 are one-time setup; after that, cutting a release is steps 8–12.
Nothing here is automatic — tagging is the only thing that starts a build.

> **v2 vs v1.** v2 has **no licence server and no company server** (zero monthly
> cost). Licences are **offline files** you sign on your own laptop with
> `tools/licence-maker`; there is **no `LICENCE_API`** and no Fly licence app to
> deploy. The relay is now an **optional** "Instant sync" add-on — deploy it only
> if you sell that add-on. Google Drive is the internet sync route (a school
> **sync account** the school owns).

> **Read first — build state.** The live Google Drive sync client, the Android app
> build + its native plugin, and live Gmail/automatic-WhatsApp sending are
> **specced-but-not-built** (deferred since P12/P14; they need your Google/Meta
> accounts + an Android toolchain — see `docs/phase-notes/phase-18.md`). Until they
> are finished and verified on your machine, v2.0.0 is a **single-PC desktop**
> release (Windows/macOS), and the Android/Drive/email steps below don't yet apply.
> iPhone support (a web app) is Phase 19.

> **Signing.** Windows and macOS ship **UNSIGNED** unless you add the `WINDOWS_*`
> / `APPLE_*` secrets. Android is `-UNSIGNED-TEST` until you add your keystore
> (steps 2–3). The release never fakes a signature.

---

### One-time setup

- [ ] **1. Confirm the app identifier.** It can NEVER change after the first
  public release. Current value: `in.vidyabudget.app`; you are considering
  `com.zuhairhussain.vidya` (OWNER-DECISIONS #1). Decide now — the Android OAuth
  client (step 5) must use whichever you pick.

- [ ] **2. Create the Android release keystore and BACK IT UP** in two safe
  places → `docs/ANDROID-SIGNING.md`. Keep the SHA-1 for step 5. *(Only when the
  Android build is ready — see the build-state note above.)*

- [ ] **3. Add the four Android secrets** to GitHub (Settings → Secrets and
  variables → Actions → **Secrets**): `ANDROID_KEYSTORE_BASE64`,
  `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS=vidya`, `ANDROID_KEY_PASSWORD`.

- [ ] **4. Mint the production licence keypair** on your laptop:
  `cd tools/licence-maker && cargo run -- init --dir <your-key-folder>`. It prints
  the **public** key and writes the **private** key + `register.csv` to the folder.
  **Back up the private key and register in two places** (README says how); never
  commit either. Add the **public** key as the GitHub **Variable**
  `LICENCE_PUBLIC_KEY`. (There is no licence server and no `gen-prod-key` — that was
  v1.)

- [ ] **5. Create the two Google OAuth clients** in your Google Cloud project
  (Desktop + Android; Android needs the step-1 package name + step-2 SHA-1) and add
  their IDs as GitHub **Variables** `GOOGLE_CLIENT_ID_DESKTOP` /
  `GOOGLE_CLIENT_ID_ANDROID` → `docs/GOOGLE-OAUTH-SETUP.md`. Also create the
  school **sync account** and **backup account** Gmails (§11). *(Needed once the
  live Drive/Gmail client is wired — build-state note.)*

- [ ] **6. (Optional) Instant sync add-on only:** deploy the relay and set the
  `RELAY_URL` Variable → `docs/DEPLOY-FLY.md`. Skip this for a normal release —
  Instant sync is off by default and the relay is not required.

- [ ] **7. Finish the live services** (or decide to ship single-PC desktop):
  run the one-time Google `drive.file` spike (`docs/phase-notes/phase-12-spike.md`)
  on real accounts + 3 devices; if it passes, build the live Drive client + sign-in
  UI, the Android native plugin, and the live Gmail/WhatsApp senders (all specced in
  the P12/P14 handoffs). If you are shipping single-PC desktop for now, note the
  limitations in the release description.

### Cut the release

- [ ] **8. Merge the release branch to `main`** (or push the branch you tag).

- [ ] **9. Tag and push:** `git tag v2.0.0 && git push --tags`. This starts
  `.github/workflows/release.yml`, which syncs the version from the tag, writes
  `build-config/release.json` from the Variables (failing clearly if any required
  one is missing), builds each platform, enforces the size gates, and creates a
  **DRAFT** release with `SHA256SUMS.txt` + `site/releases.json`.

- [ ] **10. Review the DRAFT GitHub Release.** Check: files present
  (`Vidya_2.0.0_x64-setup.exe`, `Vidya_2.0.0_universal.dmg`, and — when Android is
  ready — `Vidya_2.0.0_arm64-v8a.apk`, `Vidya_2.0.0_armeabi-v7a.apk`,
  `SHA256SUMS.txt`); each download ≤ 40 MB; the notes carry the "Unsigned build"
  banner if unsigned; the CHANGELOG `[2.0.0]` section reads correctly.

- [ ] **11. Clean-machine runs** — install on real devices and record
  date/device/result → the checklist in `docs/phase-notes/phase-18.md`
  (fresh Windows 10 → licence key → setup → invite; Android join via sync account →
  attendance offline → confirm via Drive; PC off → provisional → PC on; upgrade
  Windows v1 → v2 with real data; restore on a Mac with a transfer licence → old PC
  fenced; email + WhatsApp share from a phone; a Telugu UI walkthrough).

- [ ] **12. Publish** the release (un-draft it) once 10 and 11 pass.

---

### The build-config Variables (GitHub → Actions → Variables)

| Variable | Required? | Value | Source |
|---|---|---|---|
| `LICENCE_PUBLIC_KEY` | **required** | base64 ed25519 public key | `tools/licence-maker init` (step 4) |
| `GOOGLE_CLIENT_ID_DESKTOP` | **required** | desktop OAuth client id | Google console (step 5) |
| `GOOGLE_CLIENT_ID_ANDROID` | **required** | Android OAuth client id | Google console (step 5) |
| `LICENCE_PUBLIC_KEY_PREV` | optional | previous public key kept trusted after a key rotation | old `licence-maker` key |
| `RELAY_URL` | optional | `wss://relay.…` | your relay (step 6), Instant-sync add-on only |

These are **Variables** (public values), not secrets. The only true secrets are the
Android keystore (step 3) and the licence **private** key, which lives only in your
`licence-maker` key folder and is never part of the app build. `write-release-config.sh`
and `src-tauri/build.rs` both fail the build if a required value is missing or the
dev licence key is used. There is **no `LICENCE_API`** in v2.
