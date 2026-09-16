import { afterEach, describe, expect, it } from 'vitest';
import { closeModal, confirmDialog, openModal, toast } from './ui.js';

afterEach(() => {
  closeModal();
  const m = document.getElementById('mroot');
  if (m) m.replaceChildren();
});

describe('toast', () => {
  it('appends a toast element', () => {
    toast('Saved', { kind: 'ok' });
    const host = document.getElementById('tst');
    expect(host.querySelector('.toast.ok')).not.toBeNull();
    expect(host.textContent).toContain('Saved');
  });
});

describe('confirmDialog', () => {
  it('resolves true when confirm is clicked', async () => {
    const p = confirmDialog({ title: 'Delete?', message: 'Sure?' });
    document.querySelector('[data-ui="confirm"]').click();
    await expect(p).resolves.toBe(true);
  });

  it('resolves false when cancel is clicked', async () => {
    const p = confirmDialog({ title: 'Delete?', message: 'Sure?' });
    document.querySelector('[data-ui="cancel"]').click();
    await expect(p).resolves.toBe(false);
  });
});

describe('openModal', () => {
  it('closes an unlocked modal on Escape', () => {
    openModal({ title: 'Hello' });
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(document.getElementById('mroot').children.length).toBe(0);
  });

  it('ignores Escape when locked', () => {
    const { element } = openModal({ title: 'Locked', locked: true });
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(document.getElementById('mroot').contains(element)).toBe(true);
    closeModal();
  });

  it('escapes interpolated content', () => {
    const { element } = openModal({ title: `Sierra D'Souza <b>x</b>` });
    const h3 = element.querySelector('h3');
    // The <b> in the title is escaped to literal text, not a real element.
    expect(h3.textContent).toBe(`Sierra D'Souza <b>x</b>`);
    expect(h3.querySelector('b')).toBeNull();
    closeModal();
  });
});
