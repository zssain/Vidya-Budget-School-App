// Event delegation. Views attach ONE delegate() per root instead of inline
// handlers (see docs/UI_GUIDE.md "Events" and src/AGENTS.md).
//
//   delegate(root, {
//     save: (event, el) => { ... },   // triggered by data-action="save"
//   })
//
// Returns an unsubscribe function.

export function delegate(
  root,
  handlers,
  { events = ['click', 'change', 'input', 'submit', 'keydown'] } = {},
) {
  const isDev = typeof import.meta !== 'undefined' && import.meta.env && import.meta.env.DEV;

  function dispatch(el, event) {
    const action = el.getAttribute('data-action');
    const fn = handlers[action];
    if (typeof fn === 'function') {
      fn(event, el);
    } else if (isDev) {
      console.warn(`delegate: no handler for data-action="${action}"`);
    }
  }

  const listener = (event) => {
    const target = event.target;
    if (!target || typeof target.closest !== 'function') return;

    if (event.type === 'keydown') {
      // Only elements that opt in with data-action-key react to keys.
      const el = target.closest('[data-action][data-action-key]');
      if (!el || !root.contains(el)) return;
      if (event.key !== el.getAttribute('data-action-key')) return;
      dispatch(el, event);
      return;
    }

    const el = target.closest('[data-action]');
    if (!el || !root.contains(el)) return;
    if (event.type === 'submit') event.preventDefault();
    dispatch(el, event);
  };

  events.forEach((type) => root.addEventListener(type, listener));
  return () => events.forEach((type) => root.removeEventListener(type, listener));
}
