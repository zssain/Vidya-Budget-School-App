// MFA: the only owner page reachable at AAL1 — and only for the configured owner UUID.
import { safeOwnerRedirect } from "@vidya/billing-core";
import { notFound, redirect } from "next/navigation";
import { passwordIsFresh, requireOwner } from "@/lib/owner-guard";
import { surfaceAllows } from "@/lib/surface";
import { MfaClient } from "./mfa-client";

export default async function MfaPage({ searchParams }: { searchParams: Promise<Record<string, string | undefined>> }) {
  if (!surfaceAllows("owner")) notFound();
  const sp = await searchParams;
  const next = safeOwnerRedirect(sp.next);
  const d = await requireOwner({ allowAal1: true });
  if (!d.ok) redirect(d.reason === "not_owner" ? "/login?error=not_owner" : `/login?next=${encodeURIComponent(next)}`);
  const stepUp = sp.step_up === "1";
  if (d.owner.aal === "aal2" && !stepUp) redirect(next);
  const passwordFresh = passwordIsFresh(d.owner);
  return (
    <main className="mx-auto mt-24 max-w-md px-4">
      <div className="rounded-lg border border-slate-200 bg-white p-6 shadow-sm">
        <h1 className="mb-2 text-lg font-semibold">{stepUp ? "Confirm it's you" : d.owner.hasVerifiedTotp ? "Two-factor verification" : "Set up two-factor authentication"}</h1>
        <p className="mb-4 text-sm text-slate-600">
          {stepUp
            ? "This action is sensitive. Enter a fresh code from your authenticator app."
            : d.owner.hasVerifiedTotp
              ? "Enter the 6-digit code from your authenticator app."
              : "MFA is mandatory for the owner account. Scan the QR code with an authenticator app, then enter its code."}
        </p>
        <MfaClient enrolled={d.owner.hasVerifiedTotp} canEnrol={passwordFresh} next={next} />
      </div>
    </main>
  );
}
