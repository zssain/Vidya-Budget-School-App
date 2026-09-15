# AGENTS.md — shared crates (crates/)

Read the root `AGENTS.md` first. These rules add to it.

## Crate responsibilities (see docs/ARCHITECTURE.md for the dependency diagram)
| Crate | Contains | Must not contain |
|---|---|---|
| `vidya-core` | Domain types, money, dates, validation, fee and grade maths, usernames, amount in words, permission matrix, HLC | Any I/O, SQL, async, Tauri, network |
| `vidya-db` | SQLCipher connection, migrations, repositories (one module per table group), change-log writer | Business rules, permission checks |
| `vidya-services` | Application services: authorize, validate with core, run repositories in a transaction, write change log | Tauri types, HTTP types |
| `vidya-license` | Activation and reset code encode, decode, verify, sign (signing behind feature `signing`) | Database access |
| `vidya-sync` | Change envelopes, merge rules, protocol DTOs | Database access, network |
| `vidya-server` | axum LAN server, TLS, discovery responder, rate limits, request signing checks | Business rules (call services) |
| `vidya-client` | Discovery client, pinned HTTPS client, sync loop for Android | Business rules |
| `vidya-backup` | Backup file format, local rotation, pen drive, Google Drive upload and restore | UI |
| `vidya-export` | Excel export and import, PDF helpers | Business rules |
| `vidya-testkit` | Sample school generator, simulation harness, test fixtures (dev-dependency only) | Production code |

## Rules
- Dependencies only flow in the direction shown in `docs/ARCHITECTURE.md`. Never make `vidya-core` depend on anything else in the workspace.
- Each crate has unit tests next to the code and integration tests in `tests/`.
- Public functions have doc comments that say what they do, what errors they return and who may call them.
- Services take an `Actor` (user id, role, sections, device id), never raw ids from callers.

## Adding dependencies
External crates must already be listed in `docs/DEPENDENCIES.md`. To add a new one, stop and ask (purpose, alternatives, size, licence, last release). To wire an allowed crate into the workspace:

1. Run `cargo add <crate> -p <member>` in the member that needs it (never write a version from memory).
2. Read the resolved version in `Cargo.lock`.
3. Move it to the root `[workspace.dependencies]` pinned to that version.
4. In each member that uses it, reference it as `<crate> = { workspace = true }` (add features per member as needed).
5. Fill in the **Installed version** and **Added in** columns in `docs/DEPENDENCIES.md`.

Path dependencies between workspace crates follow the arrows in `docs/ARCHITECTURE.md` section 2 only.
