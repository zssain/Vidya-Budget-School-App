import { describe, expect, it, vi } from 'vitest';
import { render } from './html.js';
import { html } from './html.js';
import { delegate } from './dom.js';

function mount(safe) {
  const root = document.createElement('div');
  render(root, safe);
  document.body.appendChild(root);
  return root;
}

describe('delegate', () => {
  it('fires the handler when a child of a data-action element is clicked', () => {
    const root = mount(html`<button data-action="save"><span id="child">x</span></button>`);
    const save = vi.fn();
    delegate(root, { save });
    root.querySelector('#child').dispatchEvent(new Event('click', { bubbles: true }));
    expect(save).toHaveBeenCalledTimes(1);
    expect(save.mock.calls[0][1].getAttribute('data-action')).toBe('save');
  });

  it('stops firing after unsubscribe', () => {
    const root = mount(html`<button data-action="save">x</button>`);
    const save = vi.fn();
    const off = delegate(root, { save });
    off();
    root.querySelector('button').dispatchEvent(new Event('click', { bubbles: true }));
    expect(save).not.toHaveBeenCalled();
  });

  it('passes the element on input events', () => {
    const root = mount(html`<input data-action="search" />`);
    const search = vi.fn();
    delegate(root, { search });
    const input = root.querySelector('input');
    input.value = 'abc';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    expect(search).toHaveBeenCalledTimes(1);
    expect(search.mock.calls[0][1]).toBe(input);
  });

  it('only reacts to keydown on elements with a matching data-action-key', () => {
    const root = mount(html`<input data-action="go" data-action-key="Enter" />`);
    const go = vi.fn();
    delegate(root, { go });
    const input = root.querySelector('input');
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', bubbles: true }));
    expect(go).not.toHaveBeenCalled();
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    expect(go).toHaveBeenCalledTimes(1);
  });
});
