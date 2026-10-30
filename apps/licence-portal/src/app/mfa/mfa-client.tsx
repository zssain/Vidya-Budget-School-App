"use client";
import { ShieldCheck } from "lucide-react";
import { useState } from "react";

type Enrol = { factorId: string; qrCode: string; secret: string };

export function MfaClient({ enrolled, canEnrol, next }: { enrolled: boolean; canEnrol: boolean; next: string }) {
  const [enrol, setEnrol] = useState<Enrol | null>(null);
  const [code, setCode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function startEnrol() {
    setBusy(true);
    setError(null);
    const r = await fetch("/api/auth/mfa/enroll", { method: "POST", headers: { "content-type": "application/json" }, body: "{}" });
    setBusy(false);
    if (!r.ok) {
      const e = await r.json().catch(() => ({}));
      setError(e.error === "recent_password_required" ? "Sign in with your password again (within 10 minutes) to enrol." : "Could not start enrolment.");
      return;
    }
    setEnrol(await r.json());
  }

  async function verify(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    const r = await fetch("/api/auth/mfa/verify", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ code, factorId: enrol?.factorId, next }),
    });
    setBusy(false);
    const body = await r.json().catch(() => ({}));
    if (!r.ok) {
      setError(body.error === "rate_limited" ? "Too many attempts — wait a few minutes." : "That code was not accepted.");
      return;
    }
    window.location.assign(body.next ?? "/dashboard");
  }

  if (!enrolled && !enrol) {
    return (
      <div className="space-y-3">
        {error ? <p role="alert" className="text-sm text-rose-700">{error}</p> : null}
        {canEnrol ? (
          <button className="btn" onClick={startEnrol} disabled={busy}><ShieldCheck size={16} aria-hidden /> Start MFA enrolment</button>
        ) : (
          <p className="text-sm text-amber-800">For safety, enrolment requires a password sign-in in the last 10 minutes. <a className="underline" href="/login">Sign in again</a>.</p>
        )}
      </div>
    );
  }
  return (
    <form onSubmit={verify} className="space-y-3">
      {enrol ? (
        <div className="space-y-2">
          {/* Supabase returns the QR code as an SVG data URL. */}
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={enrol.qrCode} alt="TOTP enrolment QR code" width={180} height={180} className="rounded border" />
          <p className="text-xs text-slate-600">Can&apos;t scan? Secret: <span className="mono select-all">{enrol.secret}</span></p>
        </div>
      ) : null}
      <div>
        <label className="label" htmlFor="code">6-digit code</label>
        <input id="code" inputMode="numeric" autoComplete="one-time-code" pattern="\d{6}" maxLength={6} required className="input mono"
          value={code} onChange={(e) => setCode(e.target.value.replace(/\D/g, ""))} />
      </div>
      {error ? <p role="alert" className="text-sm text-rose-700">{error}</p> : null}
      <button className="btn" type="submit" disabled={busy || code.length !== 6}>Verify</button>
    </form>
  );
}
