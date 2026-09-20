export function StatCard({ label, value, note }) {
  return (
    <article className="stat">
      <div className="k">{label}</div>
      <div className="v num">{value}</div>
      {note && <div className="n">{note}</div>}
    </article>
  );
}
