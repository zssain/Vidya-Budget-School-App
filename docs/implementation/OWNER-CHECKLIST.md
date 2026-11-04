# Owner checklist — what only you can do

The code is built and tested locally (see `repair-status.md`). Everything below needs an
account, a decision, a device or an outside reviewer. Tick in order; each step names the
runbook with the exact commands. Never paste passwords, private keys or recovery codes
into a chat or a commit.

## A. Decisions (≈ 1 hour)

- [ ] Selling price(s), SKU names and which modules each includes → `infra/licensing/setup/catalogue.example.sql`.
- [ ] Domains: `owner.<domain>` (your dashboard), `buy.<domain>` (checkout), and a
      separate host for the iPhone app (`pwa_origin`). `neverworks.org` currently serves another site.
- [ ] Who in a school receives teacher leave / check-in / message data (audiences, W1).

## B. Accounts (≈ 1–2 days, plus outside waits)

- [ ] **Supabase**: project, email-only login, sign-ups OFF, TOTP MFA on → `runbooks/production-setup.md` §1–3.
- [ ] **Razorpay** test keys + webhook secret now; live keys after KYC (days to ~2 weeks) → §5.
- [ ] **Resend** account + verified sending domain (DNS, up to a day).
- [ ] **Hosting**: two portal deployments, the billing worker, the licence signer, Postgres
      with backups/PITR → §2, §4, `runbooks/operations.md`.
- [ ] **iPhone app hosting** at the PWA domain (no deployment protection) + add that origin
      to the Google OAuth web client.
- [ ] **Production licence-signing key**: generate offline, keep only on the signer host;
      its public key must match the one built into the app (`docs/implementation/licence-signer.md`).

## C. Real-device tests (≈ 3–5 days; send me every failure)

- [ ] Two PCs on one Wi-Fi: join, attendance, marks, payment on the second PC, Sync now.
- [ ] Android phone: join, PIN, auto-lock, **upgrade from an older build** (Keystore key
      migration — `docs/implementation/android-keystore.md`), including an interrupted upgrade.
- [ ] iPhone: join through Drive from the web invite link, attendance, marks, check-in,
      refused-change message, lock/unlock with PIN.
- [ ] Drive path with the school PC's Wi-Fi off (phone works through Drive only).
- [ ] One Razorpay **test** purchase end to end: pay → licence emailed → activate on a PC
      using its machine code (shown on Welcome → Set up) → `runbooks/payment-reconciliation.md` if stuck.
- [ ] Restore drill: restore a backup on a clean PC and check the data.

## D. Before selling (≈ 2–6 weeks, mostly waiting)

- [ ] Windows code-signing certificate (≈ 1–2 weeks to issue) and Apple Developer ID.
- [ ] Google OAuth app verification for Drive scopes (≈ 3–5 business days).
- [ ] Independent review of the sync/join crypto (`sync-protocol-v2.md`) — 1–3 weeks.
- [ ] Native-speaker review of Hindi and Telugu text.
- [ ] Switch Razorpay to live, set the live price, make a ₹1-style real purchase, refund it.
- [ ] Publish the first signed release (drafts only so far).

## Decided (2026-11-05)

1. **Paid, signing failed** → the order stays **Paid · Licence pending**. Use **Retry issuance**
   on the licence or order page (fresh MFA code, reason required, audited). It re-queues the
   SAME purchase, machine code and licence ID (same generation): no second charge, no new
   licence. Every failed attempt stays in the audit log and the licence page's attempt history.
2. **Signed, email failed** → **Issued · Email failed**. **Resend licence** emails the exact
   stored `.vlic` and activation key again. It never signs or issues anything new.
3. **New machine code for an issued licence** → **Transfer**: an audited replacement linked to
   the original. It is the only action that creates a new issuance (new licence ID). The old
   PC's offline licence keeps working.
4. **Captured payment** → the licence is issued and emailed straight away, once the payment,
   amount, currency, order and machine code have been verified. There is no hold or delay.
5. **Refund after delivery** → the refund is recorded and the licence is flagged for your
   review (the dashboard flag names the issued licence). The offline perpetual licence keeps
   working: there is no remote disable.
