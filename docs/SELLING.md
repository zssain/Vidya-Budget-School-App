# SELLING — the owner's one-page checklist

How you (Zuhair) sell **Vidya Budget School**: one-time UPI purchase, perpetual
offline licence, no payment provider. **You** verify each payment in your own
bank app and mint the licence on your laptop with
[`tools/licence-maker`](../tools/licence-maker/README.md).

- Support / buyers email you at: **`mohammedzuhairhussain28@gmail.com`**
- Site & downloads: **`vidya.zuhairhussain.com`** (see [`site/`](../site/))
- Price is **`[PRICE]`** until you set it · your UPI QR image is an **[OWNER]** item
- All licence commands run from `tools/licence-maker/` with `--dir <your key folder>`
  (or set `$VIDYA_LICENCE_DIR`). See the [licence-maker README](../tools/licence-maker/README.md).

---

## 1. A new sale

Do these **in order**. Never skip the bank-app check or the duplicate-UTR check.

1. **Buyer pays** — buyer scans your UPI QR (from the site's Pay / Buy page,
   [`site/`](../site/)) and pays `[PRICE]`.
2. **Buyer emails you** — UTR/UPI reference **+ school name + machine code** to
   **`mohammedzuhairhussain28@gmail.com`** (the site uses a prefilled `mailto:`).
   The machine code is on their **Welcome → Set up** screen, e.g. `7KQ2-M9XD-4TRA-P`.
3. **Confirm the money arrived** — open your **bank app** and check that this exact
   **UTR** landed for the correct **amount**. No UTR in your bank statement → no licence.
4. **Confirm the UTR is not already used** — it must not be in `register.csv`:

   ```
   licence-maker list --dir <your key folder>
   ```

   (or grep your `register.csv` for the UTR). If it's already there, it was already
   sold — do **not** issue again.
5. **Issue the licence:**

   ```
   licence-maker issue --school "<school name>" --machine <MACHINE-CODE> --utr <utr-ref> --email <buyer-email>
   ```

   The tool validates the machine-code checksum, refuses a duplicate `--utr`
   (asks for `--force`), writes the **licence key** + a **`.vlic`** file, and
   appends an `issue` row to `register.csv`.
6. **Deliver it** — email the buyer the printed **licence key** (or attach the
   **`.vlic`** file) and the **download link** (`vidya.zuhairhussain.com`, see
   [`site/`](../site/)). They activate offline on their PC.
7. **Recorded** — it is now in `register.csv` (school, UTR, buyer email, licence id).
   Then do the **back-up** step in section 4.

---

## 2. A transfer request (school moves to a new PC)

1. **Buyer sends** the **new PC's machine code** + their **licence id**.
2. **Verify identity** — match the licence id / school / buyer email against
   `register.csv` (`licence-maker list`) and confirm it's the same buyer.
3. **Issue the transfer:**

   ```
   licence-maker transfer --licence <licence-id> --machine <NEW-MACHINE-CODE>
   ```

   Keeps the same `licence_id`, appends a `transfer` row (with the old machine),
   and the app bumps `server_epoch`.
4. **Deliver** the new licence key (or `.vlic`) to the buyer for the new PC.
5. **Explain the old PC** — tell them the **old PC becomes read-only** once it next
   reads Drive: it will show **"This computer is no longer the school server"**
   (epoch fencing). They should keep using the new PC as the school server.
6. Then do the **back-up** step in section 4.

---

## 3. Refund handling — **Draft — owner review**

*Provisional; confirm with your CA and keep this consistent with the site's Refund
page in [`site/`](../site/) (written separately).*

- Intended policy: a **simple, limited refund** window **before** the buyer has
  activated / relied on the licence; once a licence has been issued and used it is
  generally **non-refundable** (perpetual software, delivered as a file).
- **Money already received in the app is never rejected.** A refund is a payment
  **back to the buyer via UPI**; you do not delete or edit the original record.
- Handle each case by email; note the outcome against the buyer in `register.csv`.

This section is provisional pending your (and your CA's) confirmation.

---

## 4. Reminders (every issue / transfer)

- **Back up the key folder AND `register.csv` in TWO places** after each
  `issue`/`transfer` — laptop + an encrypted USB or private cloud drive, passphrase
  stored separately. Losing `licence-signing.key` means you can no longer issue for
  the shipped public key; losing `register.csv` loses your customer + UTR record.
  Full instructions: [licence-maker README → "Back up the key folder"](../tools/licence-maker/README.md#back-up-the-key-folder-and-the-register-in-two-places).
- **Price** is `[PRICE]` until you set it (site copy + these docs).
- **UPI QR image** for the site is an **[OWNER]** item — add it before selling.
- **Bills** are a **simple bill with no GST line** (owner to confirm with a CA).
