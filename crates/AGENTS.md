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
