# Installing Vidya Budget School

This guide explains how to install **Vidya** on each platform, why the current
builds show a security warning (they are **unsigned** — see below), and how to
choose the PC that will act as your **school server**.

Vidya is offline-first school-management software for Indian budget schools. One
PC (usually the Principal's) becomes the school server and holds the official
record; staff join from other PCs and Android phones.

---

## Which file do I download?

| Platform | File | Runs on |
|---|---|---|
| Windows | `Vidya_1.0.0_x64-setup.exe` (NSIS installer) | Windows 10 / 11, 64-bit |
| macOS | `Vidya_1.0.0_universal.dmg` | macOS 12 or newer, Intel **and** Apple Silicon |
| Android (newer phones) | `Vidya_1.0.0_arm64-v8a.apk` | Android 7.0 or newer, 64-bit |
| Android (older / low-end phones) | `Vidya_1.0.0_armeabi-v7a.apk` | Android 7.0 or newer, 32-bit |

> If the universal macOS `.dmg` is ever too large for the size guarantee below,
> we ship two per-architecture files instead: `Vidya_1.0.0_aarch64.dmg`
> (Apple Silicon) and `Vidya_1.0.0_x64.dmg` (Intel). Pick the one for your Mac —
> Apple menu → About This Mac tells you which chip you have.

Every release also ships `SHA256SUMS.txt`. To confirm a download was not
tampered with, compare its SHA-256 against the matching line in that file
(`certutil -hashfile <file> SHA256` on Windows, `shasum -a 256 <file>` on
macOS/Linux).

---

## Minimum requirements

- **Windows:** Windows 10 or 11, 64-bit. Runs on a 4 GB RAM PC.
- **macOS:** macOS 12 (Monterey) or newer. Intel or Apple Silicon.
- **Android:** Android 7.0 or newer (API level 24), phones with 2 GB RAM.

---

## Size guarantee, and what is *not* counted

Every download is **≤ 40 MB** and, once installed, Vidya takes **≤ 50 MB** on
disk before you enter any school data. (Sizes are decimal: 40 MB = 40,000,000
bytes.)

These things are **not** part of that 40 MB / 50 MB and can add space
separately:

- **The operating system's web engine.** Windows: Microsoft **WebView2** (already
  present on most Windows 11 and updated Windows 10; auto-installed if missing —
  see the Windows section). Android: the **Android System WebView** / Chrome,
  which the OS keeps updated.
- **Your school's data** — students, fees, attendance, marks — grows over time.
- **Backups** kept on the PC and in your Google Drive.
- **Logs** (kept for 14 days, capped in size).

---

## Windows

### 1. Run the installer

Double-click `Vidya_1.0.0_x64-setup.exe`. The installer language can be English
or Hindi. It installs Vidya per-machine and adds Start-menu / desktop shortcuts.

### 2. Get past the "unsigned app" warning (SmartScreen)

**The current builds are not code-signed** (an owner decision for v1.0.0). Because
of that, Windows SmartScreen may show a blue box:

> **Windows protected your PC** — Microsoft Defender SmartScreen prevented an
> unrecognised app from starting.

This is expected for an unsigned installer. To continue:

1. Click **More info** (the small link under the message).
2. Click the **Run anyway** button that appears.

The installer then runs normally. If your organisation has locked SmartScreen
down, ask your IT admin to allow `Vidya_1.0.0_x64-setup.exe`.

### 3. WebView2 (installed automatically if needed)

Vidya's screens use Microsoft's **WebView2** runtime. The installer uses the
online **bootstrapper**: if WebView2 is missing (mostly older Windows 10), it
downloads and installs it automatically — this needs internet **once**, during
install. If WebView2 is already present (typical on Windows 11), nothing extra
is downloaded. WebView2 is part of Windows, not part of Vidya's size.

### 4. Firewall rule (for the school server)

So other PCs and phones on the school Wi-Fi/LAN can reach this computer, the
installer adds a Windows Firewall rule for Vidya on the **Private** network
profile only (not Public). You should not see a separate "Allow access?" popup,
but if Windows does ask, tick **Private networks** and allow it. Uninstalling
Vidya removes this rule.

> Only the PC that acts as the **school server** needs to be reachable. Staff PCs
> and phones make outgoing connections and do not need an incoming rule.

---

## macOS

### 1. Open the disk image

Double-click `Vidya_1.0.0_universal.dmg` (or the per-architecture DMG for your
Mac). Drag **Vidya** into the **Applications** folder.

### 2. Get past the "unidentified developer" warning (Gatekeeper)

**The current builds are not signed or notarised** (an owner decision for
v1.0.0). If you double-click Vidya the first time, macOS may say:

> "Vidya" cannot be opened because Apple could not verify it is free of malware.

To open it anyway (this is the reliable way on macOS Sonoma/Sequoia and later):

1. Double-click **Vidya** once and dismiss the warning.
2. Open **System Settings → Privacy & Security**.
3. Scroll down to the **Security** section — you'll see a line about "Vidya"
   being blocked. Click **Open Anyway** (it appears for about an hour after
   step 1).
4. Confirm with your Mac password / Touch ID, then click **Open** on the final
   prompt.

You only need to do this **once**. After that, launch Vidya normally.

> On older macOS (Ventura 13 / Sonoma 14) you can instead **right-click** (or
> Control-click) **Vidya** in Applications → **Open** → **Open**. On macOS
> Sequoia (15) that shortcut was removed, so use the Privacy & Security steps
> above.

### 3. "Local network" permission on first run

The first time Vidya runs on the machine you use as the **school server**, macOS
asks whether Vidya may find and connect to devices on your **local network**.
Click **Allow**. This is required for staff PCs and phones to reach the school
server over Wi-Fi/LAN. On staff Macs (not the server), you can allow it too — it
helps the app discover the school server automatically.

---

## Android

### 1. Choose the right APK

- Newer / mainstream phones: `Vidya_1.0.0_arm64-v8a.apk`.
- Older or very cheap phones: `Vidya_1.0.0_armeabi-v7a.apk`.

If you are not sure, install `arm64-v8a` first; if it refuses to install, use
`armeabi-v7a`.

### 2. Allow installing this app (sideload)

Vidya's APKs are installed directly (sideloaded), not from the Play Store, so
Android asks for permission the first time:

1. Open the downloaded `.apk` (from your Files app, browser downloads, or after
   copying it to the phone).
2. Android shows **"For your security, your phone is not allowed to install
   unknown apps from this source."** Tap **Settings**.
3. Turn on **Allow from this source** (this is the "Install unknown apps"
   permission, tied to the app you downloaded from — Files, Chrome, etc.).
4. Go back and tap **Install**.

Because these builds are **unsigned / test-signed**, Play Protect may also warn
you; choose to install anyway.

### 3. Stop Android from killing background sync ("Don't optimise battery")

Android's battery optimisation can stop Vidya in the background, which delays
syncing attendance, marks and payments to the school. Exempt Vidya:

1. Open **Settings → Apps → Vidya → Battery** (path varies by phone; on some it's
   **Settings → Battery → App battery management**).
2. Set Vidya to **Don't optimise** / **Unrestricted** / **No restrictions**.

Sync is still **best-effort** in the background and is never promised while the
app is closed — Vidya always shows honest status ("Offline · N waiting to send",
"Confirmed by school server") and never a fake "synced". Exempting battery
optimisation simply gives it a better chance to send while you are away.

---

## Choosing the school-server PC

One computer holds the official school record and every device syncs to it.
Choose it carefully — a phone should **not** be the school server.

A good school-server PC:

- **Is on during school hours.** Devices sync to it while staff work. If it is
  off, they fall back to Google Drive (slower) and finally hold changes on the
  device until it is back.
- **Uses a wired LAN connection** where possible — more reliable than Wi-Fi for
  the always-on server. Wi-Fi works too.
- **Does not sleep.** Turn off sleep / hibernate in the OS power settings so the
  server stays reachable. The screen may switch off; the PC must not sleep.
- **Is on Wi-Fi *without* client isolation ("AP isolation").** Many routers have
  a setting that blocks devices on the same Wi-Fi from talking to each other.
  If phones cannot find the school server on Wi-Fi, ask whoever manages the
  router to turn **client/AP isolation off** for the school network. (Phones
  can still reach the server from anywhere over the internet via the Vidya
  relay, but on-site LAN is fastest.)
- **Has the Windows firewall rule** Vidya added on the Private profile (above),
  and stays on the **Private** network profile — not **Public**.

What a typical school needs: one Windows PC as the server, kept on and awake
during the day, connected to the school router (wired if possible); the
Principal installs and activates Vidya on it, then invites staff, who join from
their own PCs (Windows/macOS) and Android phones.

---

## After installing

- **Principal / first PC:** open Vidya → **Set up a new school**, enter your
  activation code (activation needs internet once), complete the setup wizard,
  and **write down the recovery key** it shows — it is the only way to restore
  your school on a new PC.
- **Staff:** open Vidya → **Join a school** and use the invitation link, 8-digit
  code, or QR code the Principal sends you.

For everything after install — activation, recovery key, inviting staff, Google
Drive, approvals, backups, moving to a new PC — see **ADMIN-GUIDE.md**.

---

## Publisher & support

- **Published by:** Zuhair Hussain
- **Support:** mohammedzuhairhussain28@gmail.com
- **Website:** https://zuhairhussain.com

Because v1.0.0 is unsigned, Windows shows the publisher as "Unknown" and macOS
cannot verify the developer — the one-time steps above are expected and safe for
a build you downloaded from the official release page.
