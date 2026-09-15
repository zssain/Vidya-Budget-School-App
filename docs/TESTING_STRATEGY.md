# TESTING_STRATEGY.md

## Layers
| Layer | Tool | Where | Required for |
|---|---|---|---|
| Pure rules | `cargo test`, proptest | `vidya-core` | Money, fees, grades, words, usernames, HLC, validation, permissions |
| Database | `cargo test` with temp SQLCipher DBs | `vidya-db/tests` | Migrations, constraints, triggers, repositories |
| Services | `cargo test` with `vidya-testkit` sample school | `vidya-services/tests` | Every service method: allowed, denied, validation, change-log entry |
| Permission matrix | generated table test | `vidya-services/tests/permissions_matrix.rs` | Every command-level service method × principal, accountant, teacher-own, teacher-other, no session |
| License | vectors | `vidya-license/tests` | Encode, decode, tamper, device, expiry, nonce |
| Sync | simulation harness | `vidya-sync/tests`, `vidya-server/tests` | Convergence, clash rules, role filtering, idempotency |
| Backup | round trip | `vidya-backup/tests` | Encrypt, decrypt, wrong password, damaged file, v1 import |
| Frontend | Vitest + jsdom | `src/**/*.test.js` | `html` escaping, format, i18n, each view renders with mocked API and handles errors |
| API drift | node script | `scripts/check-api-drift.mjs` | Commands in Rust, JS and API.md match |
| i18n | node script | `scripts/check-i18n.mjs` | Keys in both languages, no untranslated view text |
| Desktop smoke | manual checklist | `docs/PROGRESS.md` | Each prompt |
| Windows | CI + VM checklist | `.github/workflows/ci.yml` | Every push |
| Real devices | `docs/TESTING.md` (P10.5) | Manual | Before release |

## Test data
`vidya-testkit::SampleSchool::build(seed)` creates the same school as the prototype's sample: Vaani Public School, code `vaani`, classes Nursery to VIII, users sunita (principal), sierra (teacher V-A, V-B), rakesh (teacher VI-A, VI-B, VII-A), anita (accountant), password `vidya123`, about 136 students, receipts, 6 days of attendance, Unit Test 1 marks. Deterministic for a given seed.

## Rules
- A bug fix starts with a failing test.
- Never mark a test `#[ignore]` or `.skip` without approval.
- Tests must not depend on the current date: pass a fixed clock.
- No network in unit tests; server and client tests use `127.0.0.1`.
