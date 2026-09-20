export function telHref(mobile) {
  return /^\d{10}$/.test(String(mobile)) ? `tel:+91${mobile}` : null;
}
