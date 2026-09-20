export function ChipBar({ items, value, onChange }) {
  return (
    <div className="chipbar">
      {items.map((item) => (
        <button
          key={item.value}
          className={`chip ${value === item.value ? 'on' : ''}`}
          onClick={() => onChange(item.value)}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
