# Phase 14 — Step 0 checks (two [VERIFY] gates before building)

Branch `v2/p14` (cut from `v2/p13` @ `71b0658`). Written before any Step 1+ code.
Both checks were made against **official / primary sources**, quoted below with URLs.
Rule 5: no invented API behaviour — every claim here is sourced.

---

## Check 1 — Gmail API (send) [VERIFY]  →  **STOP CONDITION for the email pipeline (Step 3)**

### 1a. Send endpoint and request shape (verified)
- **Endpoint (metadata / small message):**
  `POST https://gmail.googleapis.com/gmail/v1/users/{userId}/messages/send`
- **Endpoint (media upload, for larger MIME with attachments):**
  `POST https://gmail.googleapis.com/upload/gmail/v1/users/{userId}/messages/send`
- `userId` = the special value **`me`** for the authenticated (sync) account.
- Body = a `Message` resource whose **`raw`** field is the **base64url-encoded RFC 2822 / MIME**
  message. This matches the plan (MIME built by hand + existing `base64` crate; no Google SDK).
- Source: <https://developers.google.com/gmail/api/reference/rest/v1/users.messages/send>

### 1b. Minimum scope for sending only (verified)
- **`https://www.googleapis.com/auth/gmail.send`** — description on Google's scope page:
  *"Send email on your behalf."* It grants send **without** read/modify — the least-privilege
  choice for Vidya (matches the prompt).
- Source: <https://developers.google.com/gmail/api/auth/scopes>

### 1c. Verification requirement (the gate) — **verification IS required to publish**
- Google's scope page classifies `gmail.send` as a **SENSITIVE** scope (not merely
  non-sensitive like `drive.file`, and not "restricted" like `https://mail.google.com/` or
  `gmail.readonly`).
- Sensitive scopes **"require additional OAuth App Verification"** before an app can be
  **published (In production)**. They do **not** require the heavier *restricted-scope security
  assessment* (the annual third-party audit) — that distinction is confirmed and is the reason
  `gmail.send` was chosen over broader Gmail scopes.
- What sensitive-scope verification requires the owner to do (official list):
  1. **Verify domain ownership** of the OAuth project's authorized domains in **Google Search
     Console** (i.e. prove control of `zuhairhussain.com`).
  2. A **live, public homepage** that clearly describes the app (not just a store listing).
  3. A **privacy policy** that explicitly discloses how the app accesses/uses/stores/shares
     Google user data, linked from the OAuth consent screen — the page must be **live** (the
     current `https://zuhairhussain.com/vidya/privacy` is still a placeholder, per
     `GOOGLE-OAUTH-SETUP.md`).
  4. **Per-scope justification** for `gmail.send`.
  5. An **unlisted YouTube demo video** showing the OAuth consent flow and the actual data usage.
  6. **Branding compliance** (app name, logo, support email match the app identity).
  - **Review time: typically 3–5 business days.**
- Sources:
  <https://developers.google.com/identity/protocols/oauth2/production-readiness/sensitive-scope-verification>
  · <https://developers.google.com/gmail/api/auth/scopes>

### 1d. Interaction with the existing `drive.file` client (important)
- The Drive route already added by P12/P13 uses only the **non-sensitive** `drive.file` scope,
  so the "Vidya" OAuth app currently needs **no** verification to publish.
- **Adding `gmail.send` to the same OAuth client makes the app a "sensitive-scope" app.** Once
  the owner *Publishes* it, the sensitive-scope verification above applies to the app as a whole.
- **Testing-mode alternative (no verification):** the app can stay in **Testing** — but then only
  **up to 100 whitelisted test-user Google accounts** can connect, and each connected account's
  **refresh token expires after 7 days** (re-consent required). That is fine for the owner's own
  pilot, but **not viable for distributing to many schools** (every school's sync account would
  have to be whitelisted by the owner and re-consent weekly). So for real distribution, Publish +
  verification is the path. (`GOOGLE-OAUTH-SETUP.md` §5 already documents these testing limits.)

### 1e. Free-Gmail sending limits (verified)
- A **free consumer Gmail** account: **~500 recipients per day** (To + Cc + Bcc counted per
  recipient, not per message); exceeding it blocks sending for 1–24 h. Google Workspace (paid) is
  2,000/day.
- **Vidya's default daily cap of 400 is safely below 500** — keep 400 as the default; the setting
  may be lowered but should never be raised above the account's real limit. The sender must count
  **recipients**, not messages.
- Sources: Gmail Help "Limits for sending & getting mail"
  <https://support.google.com/mail/answer/22839> (500/day consumer; recipient-counted).

### 1f. Decision for Step 0
Per the prompt's Step 0: *"If verification is required, STOP after the report so the owner can
start the verification process; continue with Steps 2, 4, 5 (non-email parts)."*

- **Verification is required** to send email from a **published** app. → **STOP the live email
  sender (Step 3).** The owner must decide (see the handoff / OWNER-DECISIONS #13) between:
  (a) run the OAuth app in **Testing** mode for the owner's own pilot schools (no verification,
  100-account / 7-day-token limits), or (b) complete **sensitive-scope verification** and Publish
  for general distribution.
- **Continue building** Steps 1 (UPI), 2 (tap-to-WhatsApp), 4 (absence — WhatsApp path + the
  queued-email *row*), 5 (fee dues — same), 6 (circulars — non-email channels), 7 (automatic
  WhatsApp, gated on its own [VERIFY]). The `message` **outbox rows** for email are created on any
  device offline regardless; only the **school-PC sender service that actually calls Gmail** is
  gated behind the owner's verification decision. It will be **built and unit-tested against a
  fake Gmail API**, wired behind the `gmail.send` scope request, and marked **"not verified live."**

---

## Check 2 — UPI deep link `upi://pay` [VERIFY] (verified against the NPCI spec)

### 2a. Source
- **NPCI UPI Linking Specifications — Version 1.6, November 2017**, *"Common URL Specifications for
  Deep Linking and Proximity Integration"* (National Payments Corporation of India). Read in full
  from a mirrored copy of the NPCI PDF. This is NPCI's own published linking spec (the document the
  prompt asks for).
- Public mirror used: <https://www.labnol.org/files/linking.pdf> · parameter table cross-checked
  against the community mirror of the same spec
  <https://github.com/bgagan911/RandomDocs/wiki/NPCI-UPI---Specifications-for-Deep-Linking>.

### 2b. URL format
```
upi://pay?parm-name=param-value&parm-name=param-value&…
```
Vidya's per-student balance QR uses exactly:
```
upi://pay?pa=<vpa>&pn=<payee name>&am=<rupees.paise>&cu=INR&tn=<note>
```

### 2c. Parameter table (verbatim from the spec's §1.2, the tags Vidya uses)
| Tag | Type | Static | Dynamic | Meaning | Vidya use |
|---|---|---|---|---|---|
| `pa` | String | **M** | **M** | Payee VPA (`Payee→addr`) | `school.upi_id` |
| `pn` | String | **M** | **M** | Payee name (`Payee→name`) | `school.upi_name` |
| `am` | String | O | **M** | Transaction amount, **decimal**. *"If 'am' is not present then this field is editable."* | balance in rupees, 2 dp |
| `cu` | String | O | O | Currency code — *"Currently ONLY 'INR' is the supported value."* | `INR` |
| `tn` | String | O | O | *"Transaction note providing a short description of the transaction."* | e.g. "Fees Aarav VII-B" |
| `tr` | String | O | **C** | Transaction reference ID — *"Mandatory for merchant transactions and dynamic URL generation."* | **not used** (see 2e) |
| `mc` | String | O | O | Payee merchant code | not used (school is not a registered merchant) |

Encoding rules from the spec (§1.2 notes):
- Values are URL-encoded per **RFC 3986**; a **space is `%20`**.
- *"If any tag is not present it can be dropped or passed as null."* (So omit empty `tn`.)

### 2d. **Length limits — none in the NPCI spec** (honest correction to our own doc)
- The NPCI 1.6 table specifies **no maximum length for `tn` or `pn`** (lengths are given only for
  `mid`/`msid`/`mtid` = max 20 and `Query` = max 99, none of which Vidya uses).
- Therefore the **`tn ≤ 50 chars`** figure in `02-V2-CHANGES §10.1` is **Vidya's own conservative
  cap, not an NPCI rule.** Rationale to keep it: several PSP apps visibly truncate the note, and a
  shorter URL scans more reliably as a QR. **Decision:** `upi.rs` trims `tn` to **50 chars** (our
  cap, documented as ours), URL-encodes it, and drops it when empty. Recorded so nobody later
  mistakes 50 for an NPCI limit.

### 2e. **No signature / merchant tags needed** (key correctness point)
- The spec's **mandatory** `mode` / `sign` / `orgid` tags (page 6) apply to **merchant-initiated
  *signed* intents**, whose signature (SHA256withRSA512, base64) requires a key pair the acquiring
  bank uploads to UPI (§1.3). A school paying into **its own personal/business VPA does not do
  this.**
- The spec explicitly blesses the simple case (§1.1, Example 2): *"a real small one person shop
  could simply print a **static QR code containing the payee address and name** without having
  software to generate dynamic QR code… customer, after scanning, should enter the amount."*
- **Conclusion:** Vidya's QR is a plain **unsigned** `upi://pay` link (`pa`+`pn`+`am`+`cu`+`tn`).
  This is exactly how "scan a person's UPI QR to pay them" works today and is correct for a school
  recording payments manually. Vidya never becomes a PSP/merchant and never handles money.

### 2f. Amount format
- `am` is **decimal rupees** with 2 places, e.g. a ₹3,100.00 balance → `am=3100.00`.
- `upi.rs::upi_uri(vpa, name, amount_paise, note)` formats `amount_paise` as `rupees.paise` (÷100,
  always 2 dp), matching the spec's "decimal format".

---

## Summary
| Gate | Result | Effect on Phase 14 |
|---|---|---|
| **Gmail `gmail.send`** | Verified: sensitive scope, **verification required to publish** (3–5 business days; homepage + live privacy policy + Search Console domain + demo video). Free-Gmail cap ~500/day (default 400 is safe). | **STOP the live email sender (Step 3)** pending the owner's verification/testing decision (OWNER-DECISIONS #13). Build every non-email part; build the sender behind the scope + fake-Gmail tests, marked **not verified live**. |
| **UPI `upi://pay`** | Verified against **NPCI Linking Spec v1.6 (Nov 2017)**: `pa`,`pn` mandatory; `am` decimal (editable if absent); `cu`=INR only; `tn` optional; **no NPCI length limit**; **no signing needed** for a personal-VPA static/dynamic QR. | Build Step 1 as specified. `tn ≤ 50` is **our** cap (documented). Unsigned `upi://pay` link. |

**No STOP for UPI.** **STOP applies only to the live Gmail sender** — proceed with everything else.
