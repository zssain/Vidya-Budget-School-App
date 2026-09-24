# Cutting a Vidya release

How to produce the signed-or-unsigned installers for **Vidya v1.0.0** (Windows
`.exe`, macOS `.dmg`, Android `.apk`) from one git tag, and how to build each one
locally. Grounded in `docs/00-SYSTEM-CONTEXT.md §2` (deliverables and size
limits) and `prompts/P09-hardening-size-release.md §8–§10`.

> **Signing is UNSIGNED by default** (owner decision #11 in
> `README-START-HERE.md`). If signing secrets are absent, the pipeline builds
> unsigned artifacts and says so — it never fakes a signature. See
> [Signing secrets](#signing-secrets).

---

## 1. Version single-sourcing

The version lives in **one** place: the `version` field of `package.json`.
`scripts/set-version.mjs` (plain Node, no dependencies) propagates it to:

- `src-tauri/Cargo.toml` → `[package] version`
- `src-tauri/tauri.conf.json` → `version`
- The Android **`versionCode`**, computed as:

  ```
  versionCode = major * 10000 + minor * 100 + patch
  ```

  So `1.0.0` → `10000`, `1.2.3` → `10203`. The human-readable `versionName`
  stays the dotted string (`1.0.0`).

Bump a release:

```bash
# 1. Set the new version in package.json (edit the "version" field), then:
node scripts/set-version.mjs        # reads package.json, writes the files above
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "Release v1.0.0"
```

> The repo files currently read `0.1.0`; the first real release sets `1.0.0`.
> Do **not** invent other version numbers.

---

## 2. Build config (required, per environment)

Release builds read one config file, `src-tauri/build-config/<env>.json`, and
**fail if any value is missing** (see `00-SYSTEM-CONTEXT.md §10`). Copy the
example and fill it in — or, in CI, write it from repository secrets/variables:

```
src-tauri/build-config/release.json.example   # template, committed
src-tauri/build-config/release.json            # real values, NOT committed
```

Keys (all required for a release build):

| Key | What it is |
|---|---|
| `licence_api` | Base URL of the `cloud/licence` service (e.g. `https://…`) |
| `licence_public_key` | base64 ed25519 public key used to verify licences offline |
| `relay_url` | `wss://…` URL of the Vidya relay |
| `google_client_id_desktop` | Google OAuth client id (desktop installed-app flow) |
| `google_client_id_android` | Google OAuth client id (Android) |

These are **build-config values**, not signing secrets. They contain no private
keys, but the finished `release.json` is environment-specific and is not
committed.

---

## 3. The tagged release pipeline

Pushing a tag matching **`v*`** triggers `.github/workflows/release.yml`.

```bash
git tag v1.0.0
git push origin v1.0.0      # tag push is done by the owner
```

The workflow runs a matrix:

- **windows-latest** → NSIS installer.
- **macos-latest** → universal (or per-arch) DMG.
- **ubuntu-latest** → Android split APKs (JDK **17**, Android SDK/NDK from the
  phase notes — NDK **r26d** `26.3.11579264`, platform-34, build-tools 34).

Each job (Node from `.nvmrc` — currently Node 25):

1. `npm ci`
2. Run the **checks** (`tsc`, vitest, `cargo test`, clippy, `check-hex.mjs`,
   `check-deps.mjs`, `check-i18n.mjs`).
3. Run the **tests**.
4. **Build** the platform artifact via the Tauri action.
5. **Measure installed size** (`scripts/size-report.mjs`) — fails the release if
   any download exceeds 40,000,000 bytes or any installed size exceeds
   50,000,000 bytes.
6. **Rename** artifacts to the canonical names (`Vidya_1.0.0_x64-setup.exe`,
   `Vidya_1.0.0_universal.dmg` or the `_aarch64` / `_x64` pair,
   `Vidya_1.0.0_arm64-v8a.apk`, `Vidya_1.0.0_armeabi-v7a.apk`).
7. Compute **SHA-256** into `SHA256SUMS.txt`.
8. Publish a **DRAFT** GitHub Release with those files and notes from
   `CHANGELOG.md`.

The release is a **draft** — a human reviews it, confirms the signing status in
the notes, and publishes.

> Allowed CI actions only: `actions/checkout`, `actions/setup-node`,
> `dtolnay/rust-toolchain`, `actions/setup-java`, `android-actions/setup-android`,
> `tauri-apps/tauri-action`, `actions/upload-artifact`, `softprops/action-gh-release`.

---

## Signing secrets

If a platform's signing secrets are **present**, the pipeline signs (and, on
macOS, notarises). If any are **missing**, that platform builds **UNSIGNED** and
the release notes say so plainly — never a fake signature.

### Windows (Authenticode)

| Secret | Missing → |
|---|---|
| `WINDOWS_CERTIFICATE` (base64 `.pfx`) | Installer is **unsigned**. Users see SmartScreen "More info → Run anyway" (documented in INSTALL.md). |
| `WINDOWS_CERTIFICATE_PASSWORD` | Same — cannot sign without both. |

### macOS (Developer ID + notarization)

| Secret | Missing → |
|---|---|
| `APPLE_CERTIFICATE` (base64 `.p12`) | App/DMG is **unsigned**. Users right-click → **Open** (documented in INSTALL.md). |
| `APPLE_CERTIFICATE_PASSWORD` | Cannot sign. |
| `APPLE_SIGNING_IDENTITY` | Cannot sign. |
| `APPLE_ID` | Cannot **notarize** (needs a signed build first anyway). |
| `APPLE_PASSWORD` (app-specific password) | Cannot notarize. |
| `APPLE_TEAM_ID` | Cannot notarize. |

Signing needs the certificate secrets; notarization additionally needs
`APPLE_ID` + `APPLE_PASSWORD` + `APPLE_TEAM_ID`. Missing any → unsigned (or
signed-but-not-notarized), stated in the notes.

### Android (keystore)

| Secret | Missing → |
|---|---|
| `ANDROID_KEYSTORE_BASE64` (base64 `.jks`/`.keystore`) | APKs are **debug-signed** and labelled `-UNSIGNED-TEST`. |
| `ANDROID_KEYSTORE_PASSWORD` | Same. |
| `ANDROID_KEY_ALIAS` | Same. |
| `ANDROID_KEY_PASSWORD` | Same. |

With all four present, CI writes a `keystore.properties` for Gradle and produces
release-signed APKs. With any missing, it produces debug-signed
`-UNSIGNED-TEST` APKs, clearly named.

### Build-config values (also from CI secrets/variables)

Separately from signing, the release needs the `build-config` values from
[section 2](#2-build-config-required-per-environment): the **licence API URL**,
**licence public key**, **relay WSS URL**, and the two **Google OAuth client
ids**. CI writes `src-tauri/build-config/release.json` from these before
building; a missing value fails the build.

---

## 4. Building locally

### Prerequisites (all platforms)

- Node from `.nvmrc` (currently **25**), then `npm ci`.
- Rust stable toolchain.
- A filled-in `src-tauri/build-config/release.json` (see section 2).
- Set the version once: `node scripts/set-version.mjs`.

Local builds are **unsigned** unless you configure signing yourself; that is fine
for testing. Bundle output lands under `target/…/release/bundle/` (and
`src-tauri/gen/android/…/outputs/apk/` for Android). Check sizes any time with:

```bash
node scripts/size-report.mjs
```

### Windows — NSIS installer

```powershell
npm ci
npm run tauri build -- --bundles nsis
# → target\release\bundle\nsis\Vidya_<ver>_x64-setup.exe
```

The NSIS installer bundles the WebView2 **online bootstrapper**
(`embedBootstrapper` in `tauri.conf.json`), installs per-machine, offers
English/Hindi, and adds the Private-profile firewall rule via the NSIS hook.

### macOS — DMG

Universal (Intel + Apple Silicon):

```bash
npm ci
rustup target add x86_64-apple-darwin aarch64-apple-darwin
npm run tauri build -- --target universal-apple-darwin --bundles dmg
# → target/universal-apple-darwin/release/bundle/dmg/Vidya_<ver>_universal.dmg
```

If the universal DMG exceeds a size limit, build per architecture instead and
ship both (`_aarch64.dmg` + `_x64.dmg`):

```bash
npm run tauri build -- --target aarch64-apple-darwin --bundles dmg
npm run tauri build -- --target x86_64-apple-darwin  --bundles dmg
```

macOS min version is **12.0** (`tauri.conf.json`). Local DMGs are unsigned unless
you provide a Developer ID identity.

### Android — split APKs

Toolchain (from Phase 1 handoff): **JDK 17**, Android **NDK r26d**
(`26.3.11579264`), platform-34, build-tools 34, and the Rust Android targets.

```bash
rustup target add aarch64-linux-android armv7-linux-android
export JAVA_HOME=…            # JDK 17
export ANDROID_HOME=…         # SDK root
export NDK_HOME="$ANDROID_HOME/ndk/26.3.11579264"

npm ci
npm run tauri android build -- --apk --split-per-abi
# → src-tauri/gen/android/app/build/outputs/apk/…/
#     Vidya_<ver>_arm64-v8a.apk
#     Vidya_<ver>_armeabi-v7a.apk
```

`minSdk 24` (Android 7+), ABIs `arm64-v8a` + `armeabi-v7a` only, R8
`minifyEnabled` + `shrinkResources`, native libs stored uncompressed and not
extracted. Without a keystore the APKs are debug-signed test builds. Optional
`.aab`: use `npm run tauri android build -- --aab`.

---

## 5. Troubleshooting

**Android: vendored-OpenSSL cross-compile fails.** `rusqlite`'s
`bundled-sqlcipher-vendored-openssl` feature can fail to cross-compile for
Android (`openssl-sys` → `make: *** [install_dev] Error 127`, empty `LIBDIR`),
seen with **NDK r26d** in the Phase 1 spike. This is a toolchain issue, not a
Vidya bug (desktop builds the same feature fine). Fixes to try: point at a
prebuilt Android OpenSSL via `OPENSSL_DIR` /
`AARCH64_LINUX_ANDROID_OPENSSL_DIR`; try a different NDK (r25 / r27); pin
`libsqlite3-sys` / `openssl-src`. Confirm the exact working NDK against the
Phase 9 handoff before relying on it.

**Android: NDK not found.** Ensure `ANDROID_HOME`/`ANDROID_NDK_HOME` point at the
installed NDK (`…/ndk/26.3.11579264`) and JDK **17** is on `JAVA_HOME`. Use the
NDK version recorded in the phase notes — never a guessed one.

**Windows: WebView2 missing on the target PC.** The installer uses the online
**bootstrapper**, so it installs WebView2 automatically when internet is
available during install. For fully offline machines, install WebView2 first.
Never switch to the offline installer bundle (it inflates the download past the
size gate).

**Windows: SmartScreen on an unsigned build.** Expected while unsigned. Users:
**More info → Run anyway** (in INSTALL.md). To remove the warning, provide
`WINDOWS_CERTIFICATE` + `WINDOWS_CERTIFICATE_PASSWORD`.

**macOS: notarization fails or is skipped.** Notarization needs a signed build
**and** `APPLE_ID` + `APPLE_PASSWORD` (app-specific) + `APPLE_TEAM_ID`. Missing
any → the DMG ships unsigned and users right-click → **Open** (in INSTALL.md).
Check that the signing certificate is a **Developer ID Application** cert and
that the app-specific password is current.

**Size-gate failure.** `size-report.mjs` fails the release when a download >
40,000,000 bytes or an installed size > 50,000,000 bytes. Investigate the
biggest contributors (`cargo bloat`, a temporary `vite build` bundle report — as
throwaway local tools, never committed dependencies). If the **universal** macOS
`.dmg`/`.app` is what tips over, switch to **per-architecture DMGs** and update
`docs/00-SYSTEM-CONTEXT.md §2` and the download-page text. Keep the release
profile from Phase 1 (`lto`, `codegen-units = 1`, `opt-level = "s"`, `strip`,
`panic = "abort"`), keep `cargo tree -i aws-lc-rs` empty, and keep fonts subset
(the Newsreader `opsz` axis must stay).

---

## 6. Release checklist

- [ ] Set the version in `package.json`; run `node scripts/set-version.mjs`;
      confirm `Cargo.toml`, `tauri.conf.json` and the Android `versionCode` match.
- [ ] `CHANGELOG.md` updated for this version (release notes come from it).
- [ ] `src-tauri/build-config/release.json` filled in / CI secrets present for
      licence API, licence public key, relay URL, both OAuth client ids.
- [ ] Signing secrets present, **or** you accept UNSIGNED and the notes will say so.
- [ ] All checks green locally (`tsc`, vitest, `cargo test`, clippy, `check-hex`,
      `check-deps`, `check-i18n`).
- [ ] `node scripts/size-report.mjs` passes (download ≤ 40 MB, installed ≤ 50 MB)
      on every platform.
- [ ] Clean-machine runs recorded (fresh Windows 10 install/activate/setup;
      macOS join via relay; Android join + offline attendance + LAN/mobile/Drive
      sync; upgrade-over-previous; uninstall/reinstall keeps data; restore).
- [ ] Commit and push `main`; then `git tag v1.0.0 && git push origin v1.0.0`.
- [ ] Watch `release.yml`; review the **DRAFT** release: correct filenames,
      `SHA256SUMS.txt` present, signing status stated honestly.
- [ ] Publish the release; update the download page (note WebView2 / Android
      System WebView are not counted in the app size).
