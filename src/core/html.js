// Safe HTML templates. This is the ONLY module allowed to assign innerHTML
// (see eslint.config.js override and docs/UI_GUIDE.md "Safe HTML").
//
// Usage:
//   render(el, html`<b>${userValue}</b>`)   // userValue is escaped
//   html`${[html`<li>a</li>`, html`<li>b</li>`]}`  // arrays of html() joined
//   raw(trustedString)                        // wrap an already-safe string

export class SafeHtml {
  constructor(value) {
    this.value = value;
  }
  toString() {
    return this.value;
  }
}

/** Escape the five HTML-significant characters. */
export function escapeHtml(text) {
  return String(text).replace(
    /[&<>"']/g,
    (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c],
  );
}

function serialize(value) {
  if (value === null || value === undefined || value === false) return '';
  if (value instanceof SafeHtml) return value.value;
  if (Array.isArray(value)) return value.map(serialize).join('');
  return escapeHtml(value);
}

/** Tagged template. Escapes every interpolated value unless it is a SafeHtml. */
export function html(strings, ...values) {
  let out = strings[0];
  for (let i = 0; i < values.length; i++) {
    out += serialize(values[i]) + strings[i + 1];
  }
  return new SafeHtml(out);
}

/** Wrap an already-trusted SafeHtml or string as SafeHtml (for nesting). */
export function raw(value) {
  if (value instanceof SafeHtml) return value;
  if (typeof value === 'string') return new SafeHtml(value);
  throw new TypeError('raw() only accepts a SafeHtml or a string produced by html()');
}

/** Set an element's contents from a SafeHtml. Throws on anything else. */
export function render(element, safe) {
  if (!(safe instanceof SafeHtml)) {
    throw new TypeError('render() expects a SafeHtml value built with html()');
  }
  element.innerHTML = safe.value;
}
