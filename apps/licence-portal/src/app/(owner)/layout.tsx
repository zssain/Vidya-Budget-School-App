// Navigation shell only — it renders NO owner data. Authorization happens in each
// page via requireOwnerPage(), because layouts are not re-run on client navigation
// and are not a security boundary.
import {
  ClipboardList, CreditCard, FileKey2, Gauge, History, LogOut, Mail, Repeat, School, Settings,
} from "lucide-react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { surfaceAllows } from "@/lib/surface";

const NAV = [
  { href: "/dashboard", label: "Dashboard", icon: Gauge },
  { href: "/schools", label: "Schools", icon: School },
  { href: "/orders", label: "Orders", icon: ClipboardList },
  { href: "/payments", label: "Payments", icon: CreditCard },
  { href: "/licences", label: "Licences", icon: FileKey2 },
  { href: "/transfers", label: "Transfers", icon: Repeat },
  { href: "/delivery", label: "Delivery", icon: Mail },
  { href: "/audit", label: "Audit", icon: History },
  { href: "/settings", label: "Settings", icon: Settings },
];

export default function OwnerLayout({ children }: { children: React.ReactNode }) {
  if (!surfaceAllows("owner")) notFound();
  return (
    <div className="flex min-h-screen">
      <nav className="w-52 shrink-0 border-r border-slate-200 bg-white p-3" aria-label="Owner navigation">
        <div className="mb-4 px-2 text-sm font-semibold">Vidya · owner</div>
        <ul className="space-y-0.5">
          {NAV.map(({ href, label, icon: Icon }) => (
            <li key={href}>
              <Link href={href} className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm text-slate-700 hover:bg-slate-100">
                <Icon size={16} aria-hidden /> {label}
              </Link>
            </li>
          ))}
        </ul>
        <form method="post" action="/api/auth/logout" className="mt-6 px-2">
          <button className="flex items-center gap-2 text-sm text-slate-600 hover:text-slate-900" type="submit">
            <LogOut size={16} aria-hidden /> Sign out everywhere
          </button>
        </form>
      </nav>
      <main className="min-w-0 flex-1 p-6">{children}</main>
    </div>
  );
}
