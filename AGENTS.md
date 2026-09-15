# AGENTS.md — Vidya

This file is for AI coding agents (Claude Code, Codex, Cursor and others). Read it completely at the start of every session. It is short on purpose; the detail lives in `docs/`.

## 1. What Vidya is

Offline school management software for small schools in India, sold as a one-time purchase.

| App | Platform | Role |
|---|---|---|
| Vidya desktop | Windows 10/11 (.exe), macOS 11+ (.dmg) | Office computer. Local server, main database, setup, activation, backups |
| Vidya mobile | Android (.apk, .aab) | Staff phones. Client only. Syncs with the office computer on school Wi-Fi |
| Provider Tool | macOS | Used only by the seller to issue activation and reset codes |

No cloud. No internet, except the optional encrypted backup to the school's own Google Drive.

## 2. Sources of truth (in this order)

1. `AGENTS.md` (this file)
2. `docs/DECISIONS.md` — decisions that are already made
3. The spec documents in `docs/`:
   - `PRODUCT.md` — users, roles, glossary of Indian school terms
   - `ARCHITECTURE.md` — folders, crates, layers, how everything connects
   - `DEPENDENCIES.md` — the only libraries you may use
   - `DATA_MODEL.md` — the database schema
   - `PERMISSIONS.md` — who can do what
   - `API.md` — every Tauri command and LAN HTTP endpoint
   - `SYNC_PROTOCOL.md`, `LICENSE_FORMAT.md`, `BACKUP_FORMAT.md`
   - `UI_GUIDE.md` — design, wording, translation rules
   - `PLATFORMS.md` — Windows, macOS and Android differences
   - `TESTING_STRATEGY.md` — what to test and how
4. `reference/VidyaSchoolApp_step1.html` — working prototype. Source of truth for screens, wording and behaviour.
5. The prompt you were given (`docs/prompts/Px.y.md`)

If two sources disagree, or a prompt asks for something that breaks a rule above, **stop and ask**. Do not pick one silently.

## 3. How to work in every session

1. Read `AGENTS.md`, `docs/PROGRESS.md`, and the prompt file.
2. Check the prompt's "Depends on" items are marked done in `docs/PROGRESS.md`. If not, stop and say so.
3. Run `npm run verify` before changing anything (once it exists, from P1.1). If it is already failing, report that first.
4. Do the tasks **in the order written**. After each task: build, run the relevant tests, fix, then move on.
5. Run `npm run verify` at the end. It must pass.
6. Update `docs/PROGRESS.md` and, if needed, `docs/KNOWN_ISSUES.md`.
7. End with the summary format in section 9.

## 4. Never invent things (anti-hallucination rules)

You must not guess any of the following. Verify each one and say how you verified it.

| Thing | How to verify |
|---|---|
| A crate or npm package exists and does what you need | `cargo search`, `cargo add` output, `npm view <pkg>` |
| A function, type, trait or feature flag of a crate | Read the source in `~/.cargo/registry/src/*/<crate>-<version>/` or run `cargo doc -p <crate>` for the **exact version in Cargo.lock** |
| An npm package API | Read `node_modules/<pkg>/` type definitions or README for the installed version |
| A Tauri config key | The JSON schema generated in `src-tauri/gen/schemas/` or the Tauri 2 docs for the installed version |
| A Tauri CLI flag | `npm run tauri -- <command> --help` |
| A Tauri plugin and its permissions | The plugin's `permissions/` folder in the registry source |
| An Android manifest permission or API level rule | Android developer documentation; say which page |
| A macOS Info.plist key or entitlement | Apple developer documentation; say which page |
| A Windows API | The `windows` crate docs for the installed version and Microsoft Learn |

Rules:
- Never write version numbers from memory. Add dependencies with `cargo add` or `npm install`, then record the installed version in `docs/DEPENDENCIES.md`.
- Only use libraries listed in `docs/DEPENDENCIES.md`. To add one, stop and ask, explaining why and what else was considered.
- If an API you expected does not exist in the installed version, say so and propose options. Do not write code against an imagined API.
- Never claim something works unless you ran it. Write "not run" or "needs testing on Windows" instead.
- If the same error survives 3 fix attempts, stop and report the exact error text, what you tried, and your best guess. Do not rewrite the architecture to escape an error.

## 5. Things you must never do

- Weaken a permission check, validation, encryption or licensing rule to make something easier.
- Put business rules in JavaScript. JavaScript only displays data and sends user actions.
- Trust a user id, role, school code or class list sent by the frontend or a phone.
- Use floating point for money.
- UPDATE or DELETE receipts, receipt cancellations, transfer certificates or the change log.
- Store a password, backup password, private key or database key in plain text, logs or error messages.
- Put the license private key, signing keys, Google client secret or any secret in the repository.
- Delete, skip (`#[ignore]`, `.skip`) or weaken a test to make it pass. If a test is wrong, explain why before changing it.
- Leave `todo!()`, `unimplemented!()`, `unwrap()` on user data, or silent placeholder behaviour in code paths the user can reach. List any unavoidable placeholder in `docs/KNOWN_ISSUES.md`.
- Change `AGENTS.md` or `docs/DECISIONS.md` unless the prompt tells you to.
- Add a new top-level folder, crate or framework not in `docs/ARCHITECTURE.md`.
- Use `innerHTML` with anything except the `html` template helper (see `docs/UI_GUIDE.md`).

## 6. Code rules

**Rust**
- Edition and toolchain as set in `rust-toolchain.toml`.
- `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` must pass.
- Errors: `thiserror` enums in library crates; convert to `AppError` (see `docs/API.md`) at the command or HTTP boundary.
- Every write goes through a service in `crates/vidya-services`, never straight from a command to SQL.
- Every service write: authorize, validate, one database transaction, change-log entry, commit.
- All SQL uses parameters. No string-built SQL with user data.
- Platform-specific code only in `src-tauri/src/platform/` behind the `Platform` trait.

**JavaScript**
- Plain ES modules, no framework. ESLint and Prettier must pass.
- Views never call `invoke` directly. They call functions in `src/api/commands.js`.
- All user-facing text through `t('key')`.

**Data**
- Money: whole rupees, integer.
- Dates: `YYYY-MM-DD` text. Timestamps: UTC ISO 8601 text. Display in Indian formats.
- IDs: UUID v7 text for rows created on any device. Human numbers (admission, receipt, TC) come from counters.

## 7. Development environment

- The developer works on a **MacBook (Apple Silicon)** with no Windows PC.
- macOS and Android builds run locally. Windows builds and tests run in GitHub Actions on every push.
- Windows is tested by the developer in a Windows 11 ARM virtual machine (bridged network), and on a real Windows PC before release.
- Code under `#[cfg(windows)]` cannot be run locally. For every Windows-only change: make sure CI compiles and tests it, and add a "Check in Windows VM" item to `docs/PROGRESS.md` with exact steps.
- Sync is tested with the MacBook running the desktop app as server and a real Android phone on the same Wi-Fi.

## 8. Commands

| Command | What it does |
|---|---|
| `npm run verify` | Everything that must pass before a task is done: Rust fmt, clippy, tests, JS lint, JS tests, API drift check, translation check |
| `npm run tauri dev` | Run the desktop app |
| `npm run tauri android dev` | Run on a connected Android phone |
| `cargo test -p <crate>` | Test one crate |
| `./scripts/doctor.sh` | Check the developer's tools are installed |

## 9. End-of-session summary (always use this format)

```
## Summary: <prompt id> <prompt title>

### Done
- <task> — <files> — verified by <command/test>

### Not done or partly done
- <item> — <reason>

### Verification
- npm run verify: PASS/FAIL (paste the final lines)
- Other commands run: <command> → <result>

### APIs and facts I verified
- <crate/package/config key> <version> — <how verified>

### Needs checking by the developer
- [ ] On MacBook: <steps>
- [ ] In Windows VM: <steps>
- [ ] On Android phone: <steps>

### Decisions I need from you
- <question> (or "None")
```
