export function Button({ children, kind = '', className = '', ...props }) {
  return (
    <button className={`btn ${kind} ${className}`.trim()} {...props}>
      {children}
    </button>
  );
}
