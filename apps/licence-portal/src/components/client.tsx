"use client";
// Minimal client components: copy-to-clipboard and authenticated polling refresh.
import { Check, Copy } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

export function CopyButton({ text, label = "Copy" }: { text: string; label?: string }) {
  const [ok, setOk] = useState(false);
  return (
    <button
      type="button"
      className="btn-secondary"
      onClick={async () => {
        await navigator.clipboard.writeText(text);
        setOk(true);
        setTimeout(() => setOk(false), 1500);
      }}
    >
      {ok ? <Check size={14} aria-hidden /> : <Copy size={14} aria-hidden />} {ok ? "Copied" : label}
    </button>
  );
}

/** Re-runs the (owner-guarded) server render every `seconds` — no realtime channel. */
export function AutoRefresh({ seconds = 15 }: { seconds?: number }) {
  const router = useRouter();
  useEffect(() => {
    const t = setInterval(() => {
      if (document.visibilityState === "visible") router.refresh();
    }, seconds * 1000);
    return () => clearInterval(t);
  }, [router, seconds]);
  return <span className="text-xs text-slate-500">Auto-refreshes every {seconds}s</span>;
}
