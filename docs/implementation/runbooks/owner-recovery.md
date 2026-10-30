# Owner account recovery

There is deliberately **no** in-app recovery, reset or "make me owner" path. Recovery uses
the Supabase dashboard, which is protected by the owner's Supabase (organisation) login —
enable MFA on that Supabase account too, it is the root of trust.

## Lost password (authenticator still available)

1. Supabase Dashboard → Authentication → Users → the owner → **Send password recovery**, or
   set a new password directly from the dashboard.
2. Sign in at `/login` with the new password, then enter your TOTP code.
3. Optional: sign out everywhere (sidebar) to revoke other sessions.

(The portal does not implement a recovery-link landing page; if you prefer email recovery,
set the password from the dashboard instead.)

## Lost authenticator (TOTP)

1. Supabase Dashboard → Authentication → Users → the owner → Factors → **delete** the TOTP
   factor. (This is the step-up: only someone with Supabase dashboard access can do it.)
2. Sign in at `/login` with the password; within 10 minutes enrol a new factor at `/mfa`.
3. Review `/audit` for `owner_password_login` / `owner_mfa_*` entries you do not recognise.

## Suspected compromise

1. Supabase Dashboard: change the owner's password, delete all MFA factors, and sign the user
   out of all sessions (or use "Sign out everywhere" in the portal if you still can).
2. Rotate `PORTAL_DATABASE_URL` password (`ALTER ROLE vidya_portal_login PASSWORD …`) and
   redeploy the owner surface.
3. Re-enrol MFA, then review `/audit` (append-only) for owner actions with unexpected request
   ids: transfers, manual approvals, resends.
4. Payment secrets are not on the owner deployment; rotate them only if the checkout host or
   worker was also affected (Razorpay: regenerate key + webhook secret; Resend: new key).

## Replacing the owner account (new email / new person)

1. Create the new user in the Supabase dashboard (signups stay disabled), or delete the old
   user and re-run `npm run bootstrap-owner` (it refuses while other users exist).
2. Set `OWNER_AUTH_USER_ID` on the owner deployment to the new UUID and redeploy. The old
   UUID immediately loses all access, even with a valid session.
3. Delete the old Auth user. Audit history keeps the old UUID as actor.

## Supabase project lost

Create a new project, bootstrap the owner again, update `SUPABASE_URL`,
`SUPABASE_PUBLISHABLE_KEY` and `OWNER_AUTH_USER_ID`. Billing data lives in the billing
database, not in Supabase Auth, so nothing commercial is lost (if the billing schema was in
the same Supabase project, restore it from backup first — operations.md).
