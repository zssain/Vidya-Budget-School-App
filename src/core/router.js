// View registry and navigation. Navigation shows only views whose `permission`
// is in the current user's permissions (from currentUser()).

const views = new Map();
let current = { id: null, params: null };
const listeners = new Set();

/** Register a view. def: { title, render, navIcon?, nav?, permission? } */
export function registerView(id, def) {
  views.set(id, { id, nav: false, ...def });
}

export function getView(id) {
  return views.get(id);
}

export function currentView() {
  return current;
}

/** Subscribe to navigation changes. Returns an unsubscribe function. */
export function onNavigate(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function go(id, params = null) {
  current = { id, params };
  for (const fn of listeners) fn(current);
}

export function rerender() {
  for (const fn of listeners) fn(current);
}

/** Nav items visible to a user with the given permissions, in registration order. */
export function navItems(permissions) {
  return [...views.values()].filter((v) => v.nav && (!v.permission || permissions.includes(v.permission)));
}

/** May a user with these permissions open this view? */
export function canView(id, permissions) {
  const v = views.get(id);
  if (!v) return false;
  return !v.permission || permissions.includes(v.permission);
}
