# VIDYA — Owner decisions register

Every open business decision, its default currently built, where that default lives in
code/config, its status, and the owner's answer once given. Created in Phase 11 from
`02-V2-CHANGES.md §14` (plus the still-live v1 `00-SYSTEM-CONTEXT §17` items). Build the stated
default; list every **open** row in each phase handoff.

Status: **open** = default is a placeholder awaiting the owner · **decided** = the owner has
answered (or the default is the confirmed choice).

| # | Decision | Default in use | Where the default lives | Status | Owner's answer |
|---|---|---|---|---|---|
| 1 | **App identifier** (can never change after the first public release) | `in.vidyabudget.app` | `src-tauri/tauri.conf.json` (`identifier`); Android `namespace`/`applicationId` in `src-tauri/gen/android/app/build.gradle.kts`; macOS `CFBundleIdentifier` | **open** | — *(must decide before the first public release; no public release exists yet — no git tags. P11 leaves it unchanged.)* |
| 2 | **Price** | `[PRICE]` placeholder | website/site copy (P12), docs | **open** | — |
| 3 | **Attendance % rule** — does legacy Leave count as absent? | **Yes** — `% = P ÷ (P + A + L_legacy)`; new marks are P/A only | `crates/vidya-core/src/attendance.rs::percent_present` (L in denominator) | **decided** | Default confirmed in 02-V2-CHANGES §3. |
| 4 | **Attendance duty** for non-class-teachers | Class teacher only; others take attendance only via an approved/assigned duty (specific dates, auto-ends) | `crates/vidya-core` permissions/approval (attendance_duty type — P16/P17) | **decided (default)** | Per 02-V2-CHANGES §3/§10.4. |
| 5 | **Offline access lease length** | 30 days | `crates/vidya-core/src/lease.rs` | **decided (default)** | Per §8.8. |
| 6 | **Telugu logo lockup** | Show the English lockup in Telugu mode | brand kit has en/hi only; UI language logic (P13) | **open** | — |
| 7 | **Salary deduction formula** | monthly ÷ working days in month × unpaid days, rounded half-up to the rupee | `crates/vidya-core` accounts (P15) | **open** | — |
| 8 | **Leave types / quotas** | Casual 12 paid, Sick 6 paid, Unpaid | Staff HR (P17) | **open** | — |
| 9 | **Remote staff check-in** allowed? | Check-in counts "at school" only over LAN; otherwise recorded "away" for the Principal to accept | Staff HR (P17) | **open** | — |
| 10 | **Retention for students who left** | Keep (no automatic deletion without Principal confirmation) | Privacy/retention setting (P13) | **open** | — *(legal input)* |
| 11 | **Owner's UPI QR image** for the website | placeholder `[OWNER]` file | static `site/` (P12) | **open** | — |
| 12 | **Native-speaker review** of Hindi and Telugu strings | prototype samples used, flagged pending review | `src/lib/i18n/*`, prototype | **open** | — |
| 13 | **Google verification for `gmail.send`** (if required for production) | build with `gmail.send`; if sensitive-scope verification is required, STOP and report the steps/time | Communication module (P14) | **open** | — |
| 14 | **Code signing** (Windows cert, Apple Developer ID, Android keystore) | unsigned release; `signed_by: null` disclosed on the download page | release workflow, `docs/ANDROID-SIGNING.md`, `RELEASE.md` | **open** | — |
| 15 | **GST / invoice** requirements on buyer bills | simple bill, no GST line (owner to verify with a CA) | site/receipt copy (P12/P14) | **open** | — |

## Notes
- **No public release has happened** (repo has no git tags; release infra is present but unused),
  so the app identifier (#1) is not yet locked. Phase 11 reports it and does not change it.
- **Phase 12** surfaced #2 (Price `[PRICE]`), #11 (UPI QR `site/assets/upi-qr.png`), #14 (code
  signing) and #15 (GST) on the committed static `site/` and in `docs/SELLING.md` as honest
  placeholders/drafts. New build-time value: **`licence_public_keys`** (an array in
  `src-tauri/build-config/*.json`, enabling signing-key rotation) — the owner mints the first key
  with `tools/licence-maker init` and pastes its `public.key`. There is no longer a `licence_api`.
- Decisions retired by v2: the P10 online payment-provider/webhook and the licence backend/API-
  shape questions are superseded by manual UPI + offline licence files (see `02-V2-CHANGES §2/§7`
  and `docs/phase-notes/phase-10.md`).
</content>
