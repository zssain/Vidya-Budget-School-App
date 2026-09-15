# Vidya

Offline school management software for small schools in India, sold as a one-time purchase.
A Tauri 2 desktop app (Windows and macOS) with a shared web frontend, an Android client, and a Rust core — no cloud, no internet required.

## Prerequisites

Run the environment check and install anything it marks ✗ under "Core tools":

```sh
./scripts/doctor.sh
```

You need Xcode Command Line Tools, Homebrew, Node.js (see [`.nvmrc`](.nvmrc)), and Rust (stable, via rustup).
Android tooling is only needed from prompt P8.1.

## Run the app (macOS)

```sh
npm install
npm run tauri dev
```

A **Vidya** window opens showing the app version.

## Verify

Everything that must pass before a change is done — Rust fmt, clippy, tests, JS lint, JS tests, API drift and i18n checks:

```sh
npm run verify
```

To also run the licence/advisory audit locally: `VERIFY_DENY=1 npm run verify` (requires `cargo install cargo-deny --locked`).

## Get the Windows installer

There is no Windows PC in this setup; Windows is built for you by GitHub Actions on every push.

1. Open the **Actions** tab on GitHub and click the latest CI run.
2. Under **Artifacts**, download **vidya-windows-installer** and unzip it.
3. Copy the `.exe` into your Windows 11 VM and run it.
4. The installer is unsigned until P10.1, so Windows SmartScreen will warn: click **More info → Run anyway**.

## Where things are

| Path               | What                                                               |
| ------------------ | ------------------------------------------------------------------ |
| `docs/`            | The full specification (product, architecture, data model, API, …) |
| `docs/prompts/`    | The 44 build prompts, `P1.1.md` … `P10.6.md`                       |
| `docs/PROGRESS.md` | Build status and the manual checks for each prompt                 |
| `src/`             | Web frontend (shared by desktop and mobile)                        |
| `src-tauri/`       | Tauri app crate (`vidya-app`)                                      |
| `crates/`          | Rust workspace crates (core, db, services, license, sync, …)       |
| `reference/`       | The working HTML prototype                                         |

## Run a build prompt

Build steps are run one at a time from `docs/prompts/`. In Claude Code:

```
/run P1.1
```

Read the prompt's "Before you start (developer)" section first, and the "Done when (developer)" checks after.
