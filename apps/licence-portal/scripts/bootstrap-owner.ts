// ONE-TIME owner bootstrap — run by the owner on their own machine, never deployed,
// never exposed as an HTTP endpoint.
//
//   SUPABASE_URL=https://<ref>.supabase.co \
//   SUPABASE_SECRET_KEY=<server secret key, from the Supabase dashboard> \
//   npm run bootstrap-owner -- owner@example.com
//
// What it does:
//   * refuses if OWNER_AUTH_USER_ID is already set (bootstrap is one-time);
//   * refuses if the project already has users other than this email (the portal
//     is single-owner; investigate before continuing);
//   * creates ONE confirmed email/password user with a random 32-char password that
//     is printed ONCE to this terminal (store it in your password manager);
//   * prints the immutable Auth user UUID to put in OWNER_AUTH_USER_ID.
// MFA (TOTP) is then enrolled by signing in to /login → /mfa. The secret key is
// used only for this run; the deployed portal never needs it.
import { randomBytes } from "node:crypto";
import { createClient } from "@supabase/supabase-js";

async function main() {
  const email = process.argv[2]?.trim().toLowerCase();
  const url = process.env.SUPABASE_URL;
  const secret = process.env.SUPABASE_SECRET_KEY;
  if (!email || !/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email)) throw new Error("usage: npm run bootstrap-owner -- <owner-email>");
  if (!url || !secret) throw new Error("SUPABASE_URL and SUPABASE_SECRET_KEY must be set in this shell (not committed anywhere)");
  if (process.env.OWNER_AUTH_USER_ID) throw new Error("OWNER_AUTH_USER_ID is already set — bootstrap is one-time. See the owner-recovery runbook.");

  const admin = createClient(url, secret, { auth: { persistSession: false, autoRefreshToken: false, detectSessionInUrl: false } });
  const { data: list, error: listErr } = await admin.auth.admin.listUsers({ page: 1, perPage: 100 });
  if (listErr) throw new Error(`listUsers failed: ${listErr.message}`);
  const existing = list.users.find((u) => u.email?.toLowerCase() === email);
  const others = list.users.filter((u) => u.email?.toLowerCase() !== email);
  if (others.length > 0) {
    throw new Error(`the project already has ${others.length} other user(s). The owner portal must be the only account; disable signups and remove them first.`);
  }
  if (existing) {
    process.stdout.write(`User already exists. OWNER_AUTH_USER_ID=${existing.id}\n(No password was changed.)\n`);
    return;
  }
  const password = randomBytes(24).toString("base64url");
  const { data, error } = await admin.auth.admin.createUser({ email, password, email_confirm: true });
  if (error || !data.user) throw new Error(`createUser failed: ${error?.message ?? "unknown"}`);
  process.stdout.write(
    [
      "Owner account created.",
      `  OWNER_AUTH_USER_ID=${data.user.id}`,
      `  Temporary password (shown ONCE — store it in your password manager now): ${password}`,
      "Next: set OWNER_AUTH_USER_ID on the owner deployment, sign in at /login and enrol TOTP at /mfa.",
      "Do not paste this password or the secret key into chat, tickets, commits or screenshots.",
      "",
    ].join("\n"),
  );
}

main().catch((e) => {
  process.stderr.write(`bootstrap-owner: ${e instanceof Error ? e.message : String(e)}\n`);
  process.exit(1);
});
