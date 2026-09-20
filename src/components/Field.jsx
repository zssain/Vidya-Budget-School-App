export function Field({ label, error, as: Control = 'input', ...props }) {
  return (
    <label className="field">
      <span>{label}</span>
      <Control aria-invalid={Boolean(error)} {...props} />
      {error && (
        <small className="err" role="alert">
          {error}
        </small>
      )}
    </label>
  );
}
