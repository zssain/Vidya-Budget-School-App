// Public checkout. Products/prices come from the server catalogue (billing.products);
// nothing about price is sent from — or trusted from — the browser.
import { formatInr, listActiveProducts } from "@vidya/billing-core";
import { connection } from "next/server";
import { notFound } from "next/navigation";
import { db } from "@/lib/db";
import { surfaceAllows } from "@/lib/surface";
import { BuyClient } from "./buy-client";

export const metadata = { title: "Buy Vidya" };

export default async function BuyPage() {
  if (!surfaceAllows("checkout")) notFound();
  await connection(); // always render at request time (live catalogue)
  const prods = await listActiveProducts(db());
  return (
    <main className="mx-auto max-w-xl px-4 py-12">
      <h1 className="text-2xl font-semibold">Buy a Vidya licence</h1>
      <p className="mt-2 text-sm text-slate-600">
        A perpetual licence for ONE school PC. You will need that PC&apos;s <b>machine code</b>, which Vidya shows on the school PC
        during Welcome → Set up, in the form <span className="mono">XXXX-XXXX-XXXX-C</span>. Type it here exactly; we check it
        before you pay. Do not use a code from a phone or another computer.
      </p>
      {prods.length === 0 ? (
        <p className="mt-8 rounded-md border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
          Online purchase is not available yet. Please contact the seller.
        </p>
      ) : (
        <BuyClient products={prods.map((p) => ({ id: p.id, name: p.name, price: formatInr(Number(p.price_paise)), modules: p.modules }))} />
      )}
    </main>
  );
}
