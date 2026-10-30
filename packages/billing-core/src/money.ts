// Integer paise only. No floating-point arithmetic touches money anywhere.

/** Throws unless `n` is a safe, non-negative integer number of paise. */
export function assertPaise(n: unknown, label = "amount"): asserts n is number {
  if (typeof n !== "number" || !Number.isSafeInteger(n) || n < 0) {
    throw new Error(`${label} must be a non-negative integer number of paise`);
  }
}

/** Postgres bigint arrives as a string from `pg`; convert without float maths. */
export function paiseFromDb(v: string | number | bigint): number {
  const n = typeof v === "number" ? v : Number(BigInt(v));
  assertPaise(n);
  return n;
}

/** Format paise as INR using integer division only: 123456 → "₹1,234.56". */
export function formatInr(paise: number): string {
  assertPaise(paise);
  const rupees = Math.trunc(paise / 100);
  const p = paise % 100;
  const s = String(rupees);
  // Indian digit grouping: last 3 digits, then groups of 2.
  let grouped = s;
  if (s.length > 3) {
    const head = s.slice(0, -3);
    const tail = s.slice(-3);
    grouped = head.replace(/\B(?=(\d{2})+(?!\d))/g, ",") + "," + tail;
  }
  return `₹${grouped}.${String(p).padStart(2, "0")}`;
}
