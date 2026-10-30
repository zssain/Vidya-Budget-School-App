# Payment reconciliation

Principle: an order is paid only when Razorpay confirms a **captured** payment for **our**
provider order with the exact amount, currency and environment. Browser screens, customer
screenshots and UTR numbers are not proof.

## "I paid but my page still says pending"

1. Owner → Orders → search the email/school → open the order.
2. Payments table empty? Click **Reconcile with Razorpay now**. The worker asks Razorpay for
   that order's payments (server credentials) and applies any capture. Refresh after a minute.
3. Payment shows `authorized`: Razorpay has not captured it. Check auto-capture settings; an
   uncaptured authorization is auto-refunded by Razorpay. No licence is issued.
4. Still nothing: check Razorpay Dashboard → Orders → the provider order id shown on the page.
   If it was paid on a **different** order (customer started twice), that order will reconcile
   on its own row.
5. The automatic sweep also reconciles every unpaid order aged 10 min–72 h, hourly.

## Webhook delivery problems

- Razorpay Dashboard → Webhooks → delivery log: 401 = wrong `RAZORPAY_WEBHOOK_SECRET` for that
  mode; 5xx = database unavailable (Razorpay retries); 404 = URL points at the owner host.
- Missed webhooks are covered by reconciliation; you may also "resend" from the Razorpay
  dashboard — duplicates are recorded once.
- `billing.webhook_receipts.processing_state = 'failed'` with `last_error` such as
  `amount_mismatch`, `currency_mismatch`, `environment_mismatch`,
  `merchant_account_mismatch`: a review flag appears on the dashboard. Investigate in
  Razorpay; the payment is **not** applied.

## Paid, awaiting machine code

Only imported/manual orders can reach this. Ask the customer for the code shown by Vidya on
the school PC, then Orders → order → **Bind machine code and issue** (fresh MFA), or let the
customer use their status link. It can be set once; later changes are transfers.

## Refunds and disputes

Initiate refunds in the Razorpay dashboard. The webhook records them: order
`partially_refunded` / `refunded`, entitlement `refunded` (full) — transfers and re-issues are
then blocked. A dispute sets `disputed`. Each change raises a "Needs review" item. An already
issued offline perpetual licence keeps working on its PC; nothing is or can be disabled
remotely. Acknowledge the review item with a note once handled.

Two captured payments for one order (double payment): a `review_duplicate_capture` flag
appears; refund the extra payment in Razorpay. Only one licence is ever issued.

## Manual (off-platform) payments

Orders → "Record an imported / manual order" (fresh MFA), then on the order page
**MANUAL payment approval** with the bank reference, a reason and the word `MANUAL`. This is
audited, labelled as manual everywhere, excluded from "verified captured sales", and creates
an entitlement with `source='owner_manual'`. It is refused for Razorpay orders.

## Useful queries (read-only)

```sql
-- orders paid in the last day and their licence status
SELECT o.id, o.status, i.licence_id, i.status AS issuance, d.state AS delivery
  FROM billing.orders o
  LEFT JOIN billing.entitlements e ON e.order_id = o.id
  LEFT JOIN billing.licence_issuances i ON i.entitlement_id = e.id
  LEFT JOIN billing.deliveries d ON d.issuance_id = i.id
 WHERE o.updated_at > now() - interval '1 day' ORDER BY o.updated_at DESC;

-- invariant: never more than one entitlement per order (enforced by UNIQUE)
SELECT order_id, count(*) FROM billing.entitlements GROUP BY 1 HAVING count(*) > 1;
```
