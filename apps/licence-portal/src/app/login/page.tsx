import { safeOwnerRedirect } from "@vidya/billing-core";
import { LockKeyhole } from "lucide-react";
import { notFound } from "next/navigation";
import { surfaceAllows } from "@/lib/surface";

const MESSAGES: Record<string, string> = {
  invalid: "Email or password is incorrect.",
  rate_limited: "Too many attempts. Wait 15 minutes and try again.",
  not_owner: "This account is not the portal owner.",
  misconfigured: "The portal is not configured (owner UUID missing). See the setup runbook.",
};

export default async function LoginPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  if (!surfaceAllows("owner")) notFound();
  const sp = await searchParams;
  const next = safeOwnerRedirect(sp.next);
  const error = sp.error ? MESSAGES[sp.error] ?? "Sign-in failed." : null;
  return (
    <main className="mx-auto mt-24 max-w-sm px-4">
      <div className="rounded-lg border border-slate-200 bg-white p-6 shadow-sm">
        <div className="mb-4 flex items-center gap-2">
          <LockKeyhole size={18} aria-hidden />
          <h1 className="text-lg font-semibold">Vidya owner portal</h1>
        </div>
        <p className="mb-4 text-sm text-slate-600">Private owner access. There is no sign-up; the single owner account is provisioned by the operator.</p>
        {error ? <div role="alert" className="mb-3 rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-800">{error}</div> : null}
        {sp.signed_out ? <div role="status" className="mb-3 rounded-md bg-emerald-50 px-3 py-2 text-sm text-emerald-800">Signed out.</div> : null}
        <form method="post" action="/api/auth/login" className="space-y-3">
          <input type="hidden" name="next" value={next} />
          <div>
            <label className="label" htmlFor="email">Email</label>
            <input id="email" name="email" type="email" autoComplete="username" required className="input" />
          </div>
          <div>
            <label className="label" htmlFor="password">Password</label>
            <input id="password" name="password" type="password" autoComplete="current-password" required className="input" />
          </div>
          <button className="btn w-full justify-center" type="submit">Continue</button>
        </form>
        <p className="mt-4 text-xs text-slate-500">Next step: your authenticator-app code (MFA is mandatory).</p>
      </div>
    </main>
  );
}
