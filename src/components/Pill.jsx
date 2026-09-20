export function Pill({ children, kind = '' }) {
  return <span className={`pill ${kind}`}>{children}</span>;
}
export const FeePill = Pill;
