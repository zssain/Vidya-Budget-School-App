#!/usr/bin/env node
// Fails (exit 1) if owner-portal / billing-worker server dependencies or code leak
// into the Vidya app: root package.json, the root lockfile, app source (src/,
// web-pwa/) or built app bundles (dist/, dist-mobile/, web-pwa/dist/).
//
//   node scripts/check-portal-isolation.mjs
//
// Read-only. Not wired into `npm run verify` yet (that is the app's gate and is
// owned separately); add it to CI as its own step. See
// docs/implementation/owner-portal.md § Isolation.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

// PORTAL_ISOLATION_ROOT lets the check be self-tested against a scratch tree.
const root = process.env.PORTAL_ISOLATION_ROOT ?? fileURLToPath(new URL("..", import.meta.url));

// Packages that belong ONLY to apps/licence-portal, services/billing-worker or
// packages/billing-core.
const FORBIDDEN_DEPS = [
  "next", "@supabase/supabase-js", "@supabase/ssr", "@supabase/auth-js", "razorpay", "resend", "pg",
  "pg-pool", "@vidya/billing-core", "@vidya/billing-worker", "@vidya/licence-portal", "lucide-react",
  "eslint-config-next", "server-only", "tsx",
];

// Strings that must never appear in app source or app bundles.
const FORBIDDEN_TEXT = [
  /@vidya\/billing-core/, /from ["']razorpay["']/, /from ["']resend["']/, /@supabase\/(ssr|supabase-js)/,
  /RAZORPAY_KEY_SECRET/, /RAZORPAY_WEBHOOK_SECRET/, /RESEND_API_KEY/, /SUPABASE_SECRET_KEY/,
  /PORTAL_DATABASE_URL/, /BILLING_WORKER_DATABASE_URL/, /SIGNER_DATABASE_URL/, /LICENCE_SIGNING_KEY_FILE/,
  /billing\.licence_issuances/, /apps\/licence-portal/, /services\/billing-worker/,
];

const problems = [];

// 1. root package.json dependencies
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
for (const field of ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"]) {
  for (const name of Object.keys(pkg[field] ?? {})) {
    if (FORBIDDEN_DEPS.includes(name)) problems.push(`package.json ${field} contains portal/worker dependency "${name}"`);
  }
}
if (pkg.workspaces) {
  const ws = JSON.stringify(pkg.workspaces);
  if (/licence-portal|billing-worker|billing-core/.test(ws)) problems.push("package.json workspaces include the portal/worker/billing-core");
}

// 2. root lockfile must not resolve portal-only packages as app dependencies
const lockPath = join(root, "package-lock.json");
if (existsSync(lockPath)) {
  const lock = JSON.parse(readFileSync(lockPath, "utf8"));
  for (const key of Object.keys(lock.packages ?? {})) {
    if (key === "") continue;
    const name = key.replace(/^.*node_modules\//, "");
    if (["next", "@supabase/ssr", "@supabase/supabase-js", "razorpay", "resend", "pg", "@vidya/billing-core"].includes(name)) {
      problems.push(`package-lock.json resolves "${name}" (${key})`);
    }
    if (/^(apps\/licence-portal|services\/billing-worker|packages\/billing-core)/.test(key)) problems.push(`package-lock.json includes workspace ${key}`);
  }
}

// 3. app source and built bundles
const SCAN_DIRS = ["src", "web-pwa", "src-tauri/src", "dist", "dist-mobile", "web-pwa/dist"];
const SKIP = new Set(["node_modules", "target", "wasm", ".git"]);
const EXT = /\.(m?[jt]sx?|html|json|css|rs|webmanifest)$/;
function walk(dir) {
  if (!existsSync(dir)) return;
  for (const entry of readdirSync(dir)) {
    if (SKIP.has(entry)) continue;
    const p = join(dir, entry);
    const st = statSync(p);
    if (st.isDirectory()) walk(p);
    else if (EXT.test(entry) && st.size < 20 * 1024 * 1024) {
      const text = readFileSync(p, "utf8");
      for (const re of FORBIDDEN_TEXT) if (re.test(text)) problems.push(`${relative(root, p)} matches ${re}`);
    }
  }
}
for (const d of SCAN_DIRS) walk(join(root, d));

// 4. root Cargo workspace must not pull in server-only crates
const cargo = join(root, "Cargo.toml");
if (existsSync(cargo)) {
  const text = readFileSync(cargo, "utf8");
  const members = text.match(/members\s*=\s*\[([^\]]*)\]/s)?.[1] ?? "";
  if (/licence-signer|billing/.test(members)) problems.push("root Cargo.toml workspace members include a server-only crate");
}

if (problems.length) {
  console.error("portal isolation check FAILED:\n  - " + [...new Set(problems)].join("\n  - "));
  process.exit(1);
}
console.log("portal isolation check passed: no portal/worker dependencies or secrets in the Vidya app");
