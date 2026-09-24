/**
 * Vidya Budget School — full clickable prototype (React, single file).
 * Visual source of truth: design/screens/*.dc.html (the approved mock) and docs/01-MOCK-SPEC.md.
 * Every colour, font size, radius and animation below is copied from the mock.
 *
 * Fonts: load Geist, Newsreader (variable, optical sizes) and Noto Sans Devanagari in the host page.
 * Assets: set ASSET_BASE to the folder holding the brand SVGs from design/assets.
 *
 * Default export <VidyaPrototype /> shows a gallery of every screen and state.
 * With ?bare=1 in the URL it renders a single screen at 1:1 (used for video capture):
 *   window.__vidyaShow(screenId, stateIndex)
 */
import React, { useEffect, useState } from 'react';

export const ASSET_BASE = (typeof window !== 'undefined' && window.VIDYA_ASSET_BASE) || '/assets';
const A = {
  logoDark: `${ASSET_BASE}/vidya-horizontal-on-dark.svg`,
  logoLight: `${ASSET_BASE}/vidya-horizontal-on-light.svg`,
  markDark: `${ASSET_BASE}/dwaar-mark-on-dark.svg`,
};

/* ------------------------------------------------------------------ tokens */
const C = {
  bg: '#F5F7F6', outer: '#E4ECEB', surface: '#FDFDFB', welcome: '#F6F8F7', white: '#FFFFFF', panel: '#E2EAEB',
  track: '#E8EDEC', unmarked: '#FAF4E6', navy: '#0C1B38', navyDeep: '#08152B', navyRaised: '#1C3358', navyGlow: '#1A3560',
  ink: '#13233F', muted: '#56657A', onNavy: '#C9D2DE', onNavyMuted: '#9FACBF', onNavyLabel: '#8E9AAE', onNavyStrong: '#E8EDF4',
  line: '#D5DDE0', lineStrong: '#C9D3D2', lineStat: '#C9D6D5', radioOff: '#B7C2C9', accent: '#2F7479', accentHover: '#245C60',
  gold: '#C5AB7A', goldText: '#8C6A2F', goldLight: '#E6D3A8', goldLine: '#DCC495', goldOnNavy: '#E9D6AE',
  danger: '#C0392B', dangerSoft: '#D07A73', online: '#5FD0A0', onlineText: '#BFE8D6', tealSoft: '#7DB1B5',
  partBg: '#F4ECDC', partFg: '#6B5220', unBg: '#F6E4E2', unFg: '#8E2F2A', mkBg: '#E7E3F1', mkFg: '#4A3B78',
};
const acc = (a) => `rgba(47,116,121,${a})`;
const SERIF = "'Newsreader', Georgia, serif";
const SANS = "'Geist', 'Noto Sans Devanagari', 'Noto Sans Telugu', system-ui, sans-serif";
const EASE = 'cubic-bezier(.2,.8,.2,1)';
const GRAD = {
  welcome: 'radial-gradient(120% 80% at 80% 10%, #1A3560 0%, #0C1B38 55%, #08152B 100%)',
  teacher: 'radial-gradient(110% 45% at 85% 0%, #1A3560 0%, #0C1B38 60%)',
  header: 'radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)',
  success: 'radial-gradient(120% 70% at 50% 0%, #1A3560 0%, #0C1B38 60%)',
};

const CSS = `
.vd *{box-sizing:border-box}
.vd button{font-family:inherit;cursor:pointer;transition:background-color .18s ease,border-color .18s ease,color .18s ease,box-shadow .18s ease,transform .1s ease}
.vd button:active{transform:scale(.985)}
.vd input{font-family:inherit}
.vd input:focus,.vd textarea:focus{outline:none}
.vd button:focus-visible,.vd a:focus-visible,.vd input:focus-visible{box-shadow:0 0 0 3px rgba(47,116,121,.12)}
@keyframes vRise12{from{opacity:0;transform:translateY(12px)}to{opacity:1;transform:none}}
@keyframes vRise14{from{opacity:0;transform:translateY(14px)}to{opacity:1;transform:none}}
@keyframes vIn6{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
@keyframes vIn8{from{opacity:0;transform:translateY(8px)}to{opacity:1;transform:none}}
@keyframes vIn10{from{opacity:0;transform:translateY(10px)}to{opacity:1;transform:none}}
@keyframes vGrow{from{transform:scaleY(0)}to{transform:scaleY(1)}}
@keyframes vFill{from{transform:scaleX(0)}to{transform:scaleX(1)}}
@keyframes vSheet{from{transform:translateX(40px);opacity:.3}to{transform:none;opacity:1}}
@keyframes vFade{from{opacity:0}to{opacity:1}}
@keyframes vPopFee{0%{transform:scale(.5);opacity:0}60%{transform:scale(1.08);opacity:1}100%{transform:scale(1)}}
@keyframes vPopAtt{0%{transform:scale(.6);opacity:0}60%{transform:scale(1.1);opacity:1}100%{transform:scale(1)}}
@keyframes vPulse{0%,100%{box-shadow:0 0 0 0 rgba(197,171,122,0.45)}50%{box-shadow:0 0 0 6px rgba(197,171,122,0)}}
@keyframes vSpin{to{transform:rotate(360deg)}}
@media (prefers-reduced-motion: reduce){.vd *{animation:none!important;transition:none!important}}
`;
function GlobalStyle() {
  return <style dangerouslySetInnerHTML={{ __html: CSS }} />;
}

/* ------------------------------------------------------------------ icons (paths copied from the mock) */
const ICON = {
  home: '<path d="M3 11.5 12 4l9 7.5"/><path d="M5 10v10h14V10"/>',
  approvals: '<path d="M9 11l3 3 8-8"/><path d="M20 12v7a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h11"/>',
  students: '<circle cx="9" cy="8" r="3.5"/><path d="M2.5 20a6.5 6.5 0 0 1 13 0"/><path d="M16 4.5a3.5 3.5 0 0 1 0 7"/><path d="M18 14a6 6 0 0 1 3.5 6"/>',
  attendance: '<rect x="3.5" y="5" width="17" height="15" rx="2"/><path d="M8 3v4M16 3v4M3.5 10h17"/>',
  marks: '<rect x="5" y="4" width="14" height="17" rx="2"/><path d="M9 4V3h6v1M9 11h6M9 15h4"/>',
  fees: '<path d="M7 5h10M7 9h10M13 20 7 13h2.5a4 4 0 0 0 0-8"/>',
  daybook: '<path d="M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z"/><path d="M4 19V5"/>',
  staff: '<path d="M12 3 5 6v5c0 4.5 3 8 7 10 4-2 7-5.5 7-10V6z"/>',
  sync: '<path d="M20 11a8 8 0 0 0-14.5-4.5L4 8"/><path d="M4 3v5h5"/><path d="M4 13a8 8 0 0 0 14.5 4.5L20 16"/><path d="M20 21v-5h-5"/>',
  backups: '<rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9v10a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V9M10 13h4"/>',
  settings: '<path d="M4 7h10M18 7h2M4 17h4M12 17h8"/><circle cx="16" cy="7" r="2"/><circle cx="10" cy="17" r="2"/>',
  receipts: '<path d="M6 3h12v18l-3-2-3 2-3-2-3 2z"/><path d="M9 8h6M9 12h6"/>',
  requests: '<path d="M4 13h4l2 3h4l2-3h4"/><path d="M5 5h14l1 8v6H4v-6z"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
  bell: '<path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z"/><path d="M10 20a2 2 0 0 0 4 0"/>',
  chevronDown: '<path d="m6 9 6 6 6-6"/>',
  chevronRight: '<path d="m9 6 6 6-6 6"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  arrow: '<path d="M7 17 17 7M8 7h9v9"/>',
  back: '<path d="M19 12H5M11 6l-6 6 6 6"/>',
  close: '<path d="M6 6l12 12M18 6 6 18"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  help: '<circle cx="12" cy="12" r="9"/><path d="M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.3M12 17h.01"/>',
  shield: '<path d="M12 3 5 6v5c0 4.5 3 8 7 10 4-2 7-5.5 7-10V6z"/>',
  cloudOff: '<path d="M3 3l18 18"/><path d="M17.5 18H7a4.5 4.5 0 0 1-1.3-8.8M9 5.6A6 6 0 0 1 18 9a4 4 0 0 1 2.4 7.2"/>',
  qr: '<rect x="4" y="4" width="6" height="6" rx="1"/><rect x="14" y="4" width="6" height="6" rx="1"/><rect x="4" y="14" width="6" height="6" rx="1"/><path d="M14 14h2v2h-2zM18 18h2v2h-2zM14 18h2M18 14h2"/>',
  print: '<path d="M7 9V3h10v6"/><rect x="4" y="9" width="16" height="8" rx="2"/><path d="M7 14h10v7H7z"/>',
  lock: '<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/>',
  timetable: '<rect x="3.5" y="4" width="17" height="16" rx="2"/><path d="M3.5 9h17M9 9v11M15 9v11"/>',
  calendar: '<rect x="3.5" y="5" width="17" height="15" rx="2"/><path d="M8 3v4M16 3v4M3.5 10h17M8 14h.01M12 14h.01M16 14h.01"/>',
  accounts: '<path d="M4 20V11M10 20V5M16 20v-8M3 20h18"/>',
  store: '<path d="M5 8h14l-1 12H6z"/><path d="M9 8V6a3 3 0 0 1 6 0v2"/>',
  circulars: '<path d="M4 10v4h3l6 4V6L7 10z"/><path d="M17 9a4 4 0 0 1 0 6"/>',
  mail: '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="m3.5 6 8.5 7 8.5-7"/>',
  chat: '<path d="M5 18l-1.5 3.5L8 20a8 8 0 1 0-3-2z"/>',
  attach: '<path d="M20 11.5 12 19.5a5 5 0 0 1-7-7l8-8a3.5 3.5 0 0 1 5 5l-8 8a2 2 0 0 1-3-3l7-7"/>',
  image: '<rect x="3.5" y="4.5" width="17" height="15" rx="2"/><circle cx="9" cy="10" r="1.8"/><path d="m20 16-5-5-9 9"/>',
  file: '<path d="M6 3h9l4 4v14H6z"/><path d="M15 3v4h4"/>',
  login: '<path d="M10 17l5-5-5-5M15 12H3"/><path d="M14 4h5a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-5"/>',
  drive: '<path d="M7 18a4.5 4.5 0 0 1-.5-9 6 6 0 0 1 11.5 1.5A3.8 3.8 0 0 1 17.5 18z"/>',
  globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>',
};
function Icon({ name, size = 18, sw = 1.6, color = 'currentColor', style }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={sw} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" style={style}
      dangerouslySetInnerHTML={{ __html: ICON[name] }} />
  );
}
// two-tone phone tile icons (teal body, gold detail) — copied from TeacherHome.dc.html
const TILE_ICON = {
  Attendance: ['<rect x="3.5" y="5" width="17" height="15" rx="2"/><path d="M8 3v4M16 3v4M3.5 10h17"/>', '<path d="m9 15 2 2 4-4"/>'],
  Marks: ['<rect x="5" y="4" width="14" height="17" rx="2"/><path d="M9 4V3h6v1"/>', '<path d="M9 10h6M9 14h6M9 18h3"/>'],
  'Report cards': ['<path d="M6 3h9l4 4v14H6z"/><path d="M15 3v4h4"/>', '<circle cx="12" cy="12.5" r="2.5"/><path d="m10.6 14.6-.8 3.4 2.2-1 2.2 1-.8-3.4"/>'],
  'My classes': ['<path d="M3 10 12 4l9 6"/><path d="M5 10v10h14V10"/>', '<path d="M10 20v-5h4v5"/>'],
  Students: ['<circle cx="9" cy="8" r="3.5"/><path d="M2.5 20a6.5 6.5 0 0 1 13 0"/>', '<path d="M16 4.5a3.5 3.5 0 0 1 0 7M18 14a6 6 0 0 1 3.5 6"/>'],
  'My requests': ['<path d="M5 5h14l1 8v6H4v-6z"/>', '<path d="M4 13h4l2 3h4l2-3h4"/>'],
  Inbox: ['<path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z"/>', '<path d="M10 20a2 2 0 0 0 4 0"/>'],
  Sync: ['<path d="M7 18a4.5 4.5 0 0 1-.5-9 6 6 0 0 1 11.5 1.5A3.8 3.8 0 0 1 17.5 18z"/>', '<path d="m9.5 13.5 2.5-2.5 2.5 2.5M12 11v5"/>'],
  Profile: ['<circle cx="12" cy="8" r="4"/>', '<path d="M4 21a8 8 0 0 1 16 0"/>'],
};
function TileIcon({ name }) {
  const [a, b] = TILE_ICON[name];
  return (
    <svg width="34" height="34" viewBox="0 0 24 24" fill="none" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <g stroke={C.tealSoft} dangerouslySetInnerHTML={{ __html: a }} />
      <g stroke={C.gold} dangerouslySetInnerHTML={{ __html: b }} />
    </svg>
  );
}

/* ------------------------------------------------------------------ primitives */
const rise = (kind, delay = 0, dur = 520) => ({ animation: `${kind} ${dur}ms ${delay}ms ${EASE} both` });

function Dot({ color = C.accent, size = 5 }) {
  return <span style={{ width: size, height: size, borderRadius: size, background: color, flexShrink: 0 }} />;
}
function Eyebrow({ children, color = C.accent, dotColor, size = 11, track = '0.14em', dot = true, style }) {
  return (
    <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: size, fontWeight: 600, letterSpacing: track, textTransform: 'uppercase', color, ...style }}>
      {dot && <Dot color={dotColor || color} />}{children}
    </span>
  );
}
function Label({ children, dark }) {
  return <span style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: dark ? C.onNavyMuted : C.muted }}>{children}</span>;
}
function TwoLine({ l1, l2, size = 46, italicColor = C.accent, color = C.ink, track = '-0.025em', as = 'h1' }) {
  const H = as;
  return (
    <div style={{ display: 'flex', flexDirection: 'column' }}>
      <H style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: size, lineHeight: 1.05, letterSpacing: track, color }}>{l1}</H>
      {l2 && <span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: size, lineHeight: 1.1, letterSpacing: track, color: italicColor }}>{l2}</span>}
    </div>
  );
}
function Btn({ children, variant = 'primary', h = 44, full, icon, disabled, onClick, style, hl }) {
  const base = { height: h, padding: `0 ${h >= 50 ? 20 : 18}px`, borderRadius: 6, fontSize: h >= 50 ? 15 : 14, fontWeight: 500, display: 'flex', alignItems: 'center', gap: 10, justifyContent: full ? 'space-between' : 'center', width: full ? '100%' : undefined };
  const v = {
    primary: { border: 0, background: disabled ? C.lineStrong : C.accent, color: disabled ? C.muted : C.white },
    secondary: { border: `1px solid ${C.lineStrong}`, background: 'transparent', color: C.ink },
    white: { border: `1px solid ${C.line}`, background: C.white, color: C.ink },
    gold: { border: 0, background: C.gold, color: C.navy, fontWeight: 600, borderRadius: h / 2 },
    ghostNavy: { border: '1px solid rgba(255,255,255,0.3)', background: 'transparent', color: C.white, borderRadius: h / 2 },
    navy: { border: `1px solid ${C.navy}`, background: C.navy, color: C.white, borderRadius: h / 2 },
  }[variant];
  return (
    <button type="button" data-hl={hl} disabled={disabled} onClick={onClick} style={{ ...base, ...v, ...style }}>
      <span>{children}</span>{icon && <Icon name={icon} size={h >= 50 ? 18 : 16} sw={1.75} />}
    </button>
  );
}
const PILLS = {
  partpaid: [C.partBg, C.partFg], unpaid: [C.unBg, C.unFg], paid: [acc(0.12), C.accent], marks: [C.mkBg, C.mkFg],
  attendance: [acc(0.12), C.accent], details: [C.partBg, C.partFg], reversal: [C.unBg, C.unFg], neutral: [C.panel, C.ink],
};
function Pill({ tone = 'neutral', children, fixed, style }) {
  const [bg, fg] = PILLS[tone];
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', justifyContent: 'center', height: fixed ? 26 : 24, width: fixed ? 164 : undefined, padding: fixed ? 0 : '0 10px', borderRadius: 4, fontSize: 12, fontWeight: 500, whiteSpace: 'nowrap', background: bg, color: fg, justifySelf: 'start', ...style }}>{children}</span>
  );
}
function Card({ eyebrow, title, link, children, style, footer, hl }) {
  return (
    <section data-hl={hl} style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, display: 'flex', flexDirection: 'column', overflow: 'hidden', ...style }}>
      {(title || eyebrow) && (
        <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', padding: '20px 24px 16px' }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {eyebrow && <Label>{eyebrow}</Label>}
            <h2 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 26, letterSpacing: '-0.015em' }}>{title}</h2>
          </div>
          {link && <a href="#" style={{ fontSize: 13, textDecoration: 'none', color: C.ink, display: 'flex', alignItems: 'center', gap: 6, borderBottom: `1px solid ${C.ink}`, paddingBottom: 2 }}>{link}<Icon name="arrow" size={13} sw={1.75} /></a>}
        </div>
      )}
      {children}
      {footer}
    </section>
  );
}
function Field({ label, value, placeholder, focused, error, hint, mono, h = 48, hl, suffix, textarea }) {
  const border = error ? `1.5px solid ${C.danger}` : focused ? `1.5px solid ${C.accent}` : `1px solid ${C.lineStrong}`;
  const ring = error ? '0 0 0 4px rgba(192,57,43,0.14)' : focused ? `0 0 0 4px ${acc(0.12)}` : 'none';
  return (
    <div data-hl={hl} style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {label && <span style={{ fontSize: 13, fontWeight: 500 }}>{label}</span>}
      <div style={{ minHeight: h, borderRadius: 6, border, boxShadow: ring, background: C.white, padding: textarea ? '12px 14px' : '0 14px', display: 'flex', alignItems: textarea ? 'flex-start' : 'center', gap: 10, fontFamily: mono ? "'Geist', monospace" : SANS, letterSpacing: mono ? '0.06em' : 0, fontSize: 15, color: value ? C.ink : '#8A97A3', lineHeight: 1.5 }}>
        <span style={{ flexGrow: 1 }}>{value || placeholder}</span>{suffix}
      </div>
      {error && <span style={{ fontSize: 13, color: C.unFg, ...rise('vIn8', 0, 200) }}>{error}</span>}
      {hint && !error && <span style={{ fontSize: 12, color: C.muted }}>{hint}</span>}
    </div>
  );
}
function Segmented({ options, value, hl }) {
  return (
    <div data-hl={hl} role="radiogroup" style={{ display: 'grid', gridTemplateColumns: `repeat(${options.length}, minmax(0,1fr))`, border: `1px solid ${C.lineStrong}`, borderRadius: 6, overflow: 'hidden' }}>
      {options.map((o, i) => (
        <button key={o} type="button" role="radio" aria-checked={o === value} style={{ height: 44, border: 0, borderLeft: i ? `1px solid ${C.lineStrong}` : 0, fontSize: 14, fontWeight: 500, background: o === value ? C.accent : C.white, color: o === value ? C.white : C.ink }}>{o}</button>
      ))}
    </div>
  );
}
function Chip({ children, on, hl }) {
  return <button type="button" data-hl={hl} style={{ height: 36, padding: '0 14px', borderRadius: 18, fontSize: 13, fontWeight: 500, border: `1px solid ${on ? C.navy : C.lineStrong}`, background: on ? C.navy : 'transparent', color: on ? C.white : C.ink }}>{children}</button>;
}
function PinDots({ filled = 0, total = 6, dark }) {
  return (
    <div style={{ display: 'flex', gap: 14 }}>
      {Array.from({ length: total }).map((_, i) => (
        <span key={i} style={{ width: 16, height: 16, borderRadius: 8, border: `1.5px solid ${i < filled ? C.accent : dark ? 'rgba(255,255,255,0.3)' : C.radioOff}`, background: i < filled ? C.accent : 'transparent', transition: 'background-color .18s ease' }} />
      ))}
    </div>
  );
}
function Toast({ children, bottom = 108, dark = true }) {
  return (
    <div role="status" style={{ position: 'absolute', left: 16, right: 16, bottom, background: C.navy, color: C.white, borderRadius: 10, padding: '12px 14px', fontSize: 14, display: 'flex', alignItems: 'center', gap: 10, boxShadow: '0 10px 30px rgba(11,26,51,0.3)', ...rise('vIn10', 0, 220) }}>
      <Icon name="check" size={18} sw={2} color={C.gold} />{children}
    </div>
  );
}
function FakeQR({ size = 184, seed = 7 }) {
  // Decorative stand-in for the real invitation QR (the app renders a real one with the qrcode crate).
  const n = 25; const cell = size / n; const cells = [];
  let s = seed;
  const rnd = () => { s = (s * 9301 + 49297) % 233280; return s / 233280; };
  const finder = (x, y) => x < 7 && y < 7 || x >= n - 7 && y < 7 || x < 7 && y >= n - 7;
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    if (finder(x, y)) continue;
    if (rnd() > 0.52) cells.push(<rect key={`${x}-${y}`} x={x * cell} y={y * cell} width={cell} height={cell} fill={C.navy} />);
  }
  const F = ({ x, y }) => (
    <g><rect x={x * cell} y={y * cell} width={7 * cell} height={7 * cell} fill={C.navy} /><rect x={(x + 1) * cell} y={(y + 1) * cell} width={5 * cell} height={5 * cell} fill={C.white} /><rect x={(x + 2) * cell} y={(y + 2) * cell} width={3 * cell} height={3 * cell} fill={C.navy} /></g>
  );
  return <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-label="Invitation QR code">{cells}<F x={0} y={0} /><F x={n - 7} y={0} /><F x={0} y={n - 7} /></svg>;
}

/* ------------------------------------------------------------------ desktop shell */
const NAV_PRINCIPAL = [
  ['Overview', [['home', 'Home'], ['approvals', 'Approvals', 4]]],
  ['Academics', [['students', 'Students'], ['attendance', 'Attendance'], ['marks', 'Marks & exams'], ['timetable', 'Timetable'], ['calendar', 'Calendar']]],
  ['Finance', [['fees', 'Fees'], ['accounts', 'Accounts'], ['store', 'School store']]],
  ['School', [['staff', 'Staff & access'], ['circulars', 'Circulars'], ['backups', 'Backups'], ['settings', 'Settings']]],
];
const NAV_ACCOUNTANT = [[null, [['home', 'Home'], ['fees', 'Collect fee'], ['students', 'Students & admissions'], ['receipts', 'Receipts'], ['daybook', 'Day book'], ['requests', 'My requests']]]];
function Sidebar({ role = 'principal', active = 'home', badge = 4 }) {
  const groups = role === 'principal' ? NAV_PRINCIPAL : NAV_ACCOUNTANT;
  const user = role === 'principal' ? ['PS', 'Priya Sharma', 'Principal'] : ['SP', 'Suresh Patel', 'Accountant'];
  return (
    <aside style={{ width: 256, flexShrink: 0, background: C.navy, color: C.white, display: 'flex', flexDirection: 'column', padding: '26px 16px 20px', gap: 22 }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4, padding: '0 10px' }}>
        <img src={A.logoDark} alt="Vidya Budget School" style={{ width: 160, height: 52, display: 'block' }} />
        <span style={{ fontSize: 12, color: C.onNavyMuted }}>Saraswati Public School</span>
      </div>
      <nav aria-label="Main" style={{ display: 'flex', flexDirection: 'column', gap: role === 'principal' ? 14 : 2 }}>
        {groups.map(([g, items], gi) => (
          <div key={gi} style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            {g && <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: C.onNavyLabel, padding: '0 10px 8px' }}>{g}</div>}
            {items.map(([id, label, count]) => {
              const on = id === active;
              return (
                <a key={id} href="#" style={{ display: 'flex', alignItems: 'center', gap: 11, height: 38, padding: '0 10px', borderRadius: 6, background: on ? 'rgba(255,255,255,0.07)' : 'transparent', color: on ? C.white : C.onNavy, textDecoration: 'none', fontWeight: 500 }}>
                  <Icon name={id} color={on ? C.gold : 'currentColor'} /><span>{label}</span>
                  {count && badge ? <span style={{ marginLeft: 'auto', minWidth: 20, height: 20, padding: '0 6px', borderRadius: 10, background: C.gold, color: C.navy, fontSize: 11, fontWeight: 700, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>{badge}</span>
                    : on ? <span style={{ marginLeft: 'auto', width: 5, height: 5, borderRadius: 3, background: C.gold }} /> : null}
                </a>
              );
            })}
          </div>
        ))}
      </nav>
      <div style={{ marginTop: 'auto', display: 'flex', flexDirection: 'column', gap: 14 }}>
        {role === 'principal' && (
          <div style={{ border: '1px solid rgba(255,255,255,0.12)', borderRadius: 10, padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 6 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, fontWeight: 500 }}><span style={{ width: 7, height: 7, borderRadius: 4, background: C.online, boxShadow: '0 0 0 3px rgba(95,208,160,0.18)' }} />School server online</div>
            <div style={{ fontSize: 12, color: C.onNavyMuted }}>This PC · 9 devices</div>
          </div>
        )}
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '0 6px' }}>
          <div style={{ width: 34, height: 34, borderRadius: 17, background: C.navyRaised, color: C.goldLight, display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 600, fontSize: 13 }}>{user[0]}</div>
          <div style={{ display: 'flex', flexDirection: 'column' }}><span style={{ fontWeight: 500, fontSize: 13 }}>{user[1]}</span><span style={{ fontSize: 12, color: C.onNavyMuted }}>{user[2]}</span></div>
        </div>
      </div>
    </aside>
  );
}
function SyncPill({ children = 'All changes confirmed · 12 s ago', hl }) {
  return (
    <span data-hl={hl} style={{ height: 34, padding: '0 12px', borderRadius: 17, background: acc(0.1), color: C.accent, fontSize: 13, fontWeight: 500, display: 'flex', alignItems: 'center', gap: 8 }}>
      <span style={{ width: 6, height: 6, borderRadius: 3, background: C.accent }} />{children}
    </span>
  );
}
function Header() {
  return (
    <header style={{ height: 64, flexShrink: 0, borderBottom: `1px solid ${C.line}`, display: 'flex', alignItems: 'center', gap: 14, padding: '0 40px' }}>
      <button type="button" style={{ height: 36, padding: '0 12px', borderRadius: 6, border: `1px solid ${C.line}`, background: C.surface, color: C.ink, fontSize: 13, fontWeight: 500, display: 'flex', alignItems: 'center', gap: 8 }}>Session 2026–27<Icon name="chevronDown" size={15} sw={1.75} /></button>
      <div style={{ position: 'relative', width: 400 }}>
        <Icon name="search" size={16} sw={1.75} color={C.muted} style={{ position: 'absolute', left: 12, top: 11 }} />
        <div style={{ height: 38, borderRadius: 6, border: `1px solid ${C.line}`, background: C.surface, padding: '0 16px 0 36px', display: 'flex', alignItems: 'center', fontSize: 14, color: '#8A97A3' }}>Search students, receipts, staff…</div>
      </div>
      <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 10 }}>
        <SyncPill hl="syncpill" />
        <button type="button" aria-label="Notifications" style={{ position: 'relative', width: 38, height: 38, borderRadius: 19, border: `1px solid ${C.line}`, background: C.surface, color: C.ink, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Icon name="bell" size={17} /><span style={{ position: 'absolute', top: 7, right: 8, width: 7, height: 7, borderRadius: 4, background: C.gold }} />
        </button>
      </div>
    </header>
  );
}
function Desktop({ children, role, active, w = 1440, h = 1080, header = true, badge }) {
  return (
    <div className="vd" style={{ width: w, height: h, display: 'flex', background: C.bg, color: C.ink, fontFamily: SANS, fontSize: 14, overflow: 'hidden', position: 'relative' }}>
      <Sidebar role={role} active={active} badge={badge} />
      <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        {header && <Header />}
        {children}
      </div>
    </div>
  );
}
function PageTitle({ eyebrow, l1, l2, actions }) {
  return (
    <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', gap: 24, ...rise('vRise12') }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        <Eyebrow>{eyebrow}</Eyebrow>
        <TwoLine l1={l1} l2={l2} />
      </div>
      {actions && <div style={{ display: 'flex', gap: 10 }}>{actions}</div>}
    </div>
  );
}

/* ------------------------------------------------------------------ split layout (Welcome / setup / PIN) */
const SETUP_STEPS = ['Activate', 'School', 'Session & classes', 'You', 'Recovery key', 'Ready'];
function Split({ children, right, w = 1440, h = 960 }) {
  return (
    <div className="vd" style={{ width: w, height: h, padding: 20, display: 'flex', gap: 20, background: C.outer, color: C.ink, fontFamily: SANS, fontSize: 14 }}>
      <section style={{ width: 600, flexShrink: 0, background: C.welcome, borderRadius: 22, boxShadow: '0 1px 2px rgba(11,26,51,0.06)', padding: '36px 56px', display: 'flex', flexDirection: 'column' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <img src={A.logoLight} alt="Vidya Budget School" style={{ width: 190, height: 62, display: 'block' }} />
          <a href="#" style={{ fontSize: 13, color: C.muted, textDecoration: 'none', display: 'flex', alignItems: 'center', gap: 6 }}><Icon name="help" size={15} sw={1.75} />Help</a>
        </div>
        <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 22, maxWidth: 420, ...rise('vRise14') }}>{children}</div>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, color: C.muted }}><span>© 2026 Zuhair Hussain</span><span style={{ display: 'flex', gap: 20 }}><a href="#" style={{ color: C.muted, textDecoration: 'none' }}>Privacy</a><a href="#" style={{ color: C.muted, textDecoration: 'none' }}>Terms</a></span></div>
      </section>
      <section style={{ flexGrow: 1, position: 'relative', overflow: 'hidden', borderRadius: 22, background: GRAD.welcome, color: C.white, padding: '44px 52px 32px', display: 'flex', flexDirection: 'column', gap: 26 }}>{right}</section>
    </div>
  );
}
function StepsPanel({ current }) {
  const idx = SETUP_STEPS.indexOf(current);
  return (
    <>
      <span style={{ fontSize: 11, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: C.onNavyMuted }}>Setting up your school</span>
      <div style={{ ...rise('vRise14', 120, 600) }}>
        <h2 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 56, lineHeight: 1.04, letterSpacing: '-0.03em' }}>A few short steps.</h2>
        <span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 56, lineHeight: 1.1, letterSpacing: '-0.03em', color: C.gold }}>You can come back anytime.</span>
      </div>
      <ol style={{ listStyle: 'none', margin: '8px 0 0', padding: 0, display: 'flex', flexDirection: 'column', gap: 4, maxWidth: 520 }}>
        {SETUP_STEPS.map((s, i) => {
          const done = i < idx, on = i === idx;
          return (
            <li key={s} style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '14px 18px', borderRadius: 10, background: on ? 'rgba(255,255,255,0.07)' : 'transparent', border: on ? '1px solid rgba(255,255,255,0.12)' : '1px solid transparent' }}>
              <span style={{ width: 30, height: 30, borderRadius: 15, display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: SERIF, fontSize: 16, flexShrink: 0, background: done ? C.gold : 'transparent', color: done ? C.navy : on ? C.gold : C.onNavyMuted, border: done ? 0 : `1px solid ${on ? C.gold : 'rgba(255,255,255,0.25)'}` }}>
                {done ? <Icon name="check" size={15} sw={2.2} /> : i + 1}
              </span>
              <span style={{ fontSize: 16, color: on ? C.white : done ? C.onNavy : C.onNavyMuted, fontWeight: on ? 500 : 400 }}>{s}</span>
              {on && <span style={{ marginLeft: 'auto', width: 5, height: 5, borderRadius: 3, background: C.gold }} />}
            </li>
          );
        })}
      </ol>
      <div style={{ marginTop: 'auto', display: 'flex', justifyContent: 'space-between', paddingTop: 16, borderTop: '1px solid rgba(255,255,255,0.14)', fontSize: 13, color: C.onNavy }}>
        <span>Your data stays on this computer.</span><span>Encrypted from the start.</span>
      </div>
    </>
  );
}
function BrandPanel() {
  return (
    <>
      <span style={{ fontSize: 11, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: C.onNavyMuted }}>School management for Indian schools</span>
      <div style={rise('vRise14', 120, 600)}>
        <h2 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 64, lineHeight: 1.02, letterSpacing: '-0.03em' }}>Every class. Every rupee.</h2>
        <span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 64, lineHeight: 1.05, letterSpacing: '-0.03em', color: C.gold }}>One clear record.</span>
      </div>
      <p style={{ margin: 0, fontSize: 15, lineHeight: 1.6, color: C.onNavy, maxWidth: 460 }}>Attendance, marks and fees are saved on the phone first, then confirmed by your school's own computer.</p>
      <div style={{ flexGrow: 1, border: '1px solid rgba(255,255,255,0.14)', borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', minHeight: 0 }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 22, ...rise('vRise14', 200, 700) }}>
          <img src={A.markDark} alt="" style={{ width: 220, height: 220, display: 'block' }} />
          <span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 22, color: C.gold }}>A door to every classroom.</span>
        </div>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', paddingTop: 16, borderTop: '1px solid rgba(255,255,255,0.14)', fontSize: 13, color: C.onNavy }}><span>Attendance.</span><span>Marks.</span><span>Fees.</span><span>Approvals.</span></div>
    </>
  );
}

/* ================================================================== SCREENS: setup */
export function WelcomeScreen({ mode = 'setup' }) {
  const opts = [['setup', 'Set up my school', 'For the Principal · uses the activation code from your purchase'], ['join', 'Join my school', 'For teachers and accountants · uses your invitation'], ['recover', 'Recover an existing school', 'Move Vidya to a new computer from a backup']];
  const cta = { setup: 'Activate and continue', join: 'Join school', recover: 'Start recovery' }[mode];
  return (
    <Split right={<BrandPanel />}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Eyebrow>Your school on Vidya</Eyebrow>
        <span style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 12, color: C.muted }}><Icon name="shield" size={14} sw={1.75} />Encrypted on your devices</span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        <h1 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 60, lineHeight: 1, letterSpacing: '-0.03em' }}>Namaste.</h1>
        <p style={{ margin: 0, fontSize: 15, lineHeight: 1.55, color: C.muted }}>How would you like to begin? You only need an activation code when you are setting up a new school.</p>
      </div>
      <div data-hl="options" role="radiogroup" style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {opts.map(([id, t, s]) => <RadioCard key={id} on={id === mode} title={t} sub={s} hl={id} />)}
      </div>
      {mode === 'setup' && <Field label="Activation code" placeholder="VIDYA-XXXX-XXXX-XXXX" mono focused hint="It is in your purchase email and on your account page." hl="code" />}
      {mode === 'join' && <Field label="Invitation link or code" placeholder="Paste the link your Principal sent" focused hint="On a phone you can also scan the invitation QR code." />}
      <Btn full h={50} icon="arrow" hl="cta">{cta}</Btn>
      <div style={{ height: 1, background: C.line }} />
      <span style={{ fontSize: 13, color: C.muted, textAlign: 'center' }}>Already set up on this device? <a href="#" style={{ color: C.ink, fontWeight: 600, textDecoration: 'none' }}>Sign in</a></span>
    </Split>
  );
}
function RadioCard({ on, title, sub, hl }) {
  return (
    <button type="button" role="radio" aria-checked={on} data-hl={hl} style={{ display: 'flex', alignItems: 'center', gap: 14, padding: '14px 16px', borderRadius: 8, border: `1.5px solid ${on ? C.accent : C.line}`, background: on ? acc(0.06) : C.white, textAlign: 'left' }}>
      <span style={{ width: 18, height: 18, borderRadius: 9, flexShrink: 0, border: on ? `5px solid ${C.accent}` : `1.5px solid ${C.radioOff}`, background: C.white }} />
      <span style={{ display: 'flex', flexDirection: 'column', gap: 3 }}><span style={{ fontSize: 15, fontWeight: 600, color: C.ink }}>{title}</span><span style={{ fontSize: 13, color: C.muted }}>{sub}</span></span>
    </button>
  );
}

export function ActivateScreen({ state = 'entered' }) {
  const code = 'VIDYA-7KQ2-M9XD-4TRA';
  return (
    <Split right={<StepsPanel current="Activate" />}>
      <Eyebrow>Step 1 of 6</Eyebrow>
      <TwoLine l1="Activate your school." l2="One code, once." size={48} track="-0.03em" />
      <p style={{ margin: 0, fontSize: 15, lineHeight: 1.55, color: C.muted }}>Enter the code from your purchase email. Activation needs the internet once; after that Vidya works offline.</p>
      <Field label="Activation code" value={code} mono focused={state !== 'done'} hl="code" suffix={state === 'done' ? <Icon name="check" size={18} sw={2} color={C.accent} /> : null} />
      {state === 'checking' && (
        <Btn full h={50} style={{ justifyContent: 'center' }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: 10 }}><span style={{ width: 16, height: 16, border: '2px solid rgba(255,255,255,0.35)', borderTopColor: C.white, borderRadius: 8, animation: 'vSpin .8s linear infinite' }} />Checking your code…</span>
        </Btn>
      )}
      {state === 'entered' && <Btn full h={50} icon="arrow" hl="cta">Activate</Btn>}
      {state === 'done' && (
        <>
          <div data-hl="licence" style={{ borderRadius: 10, background: C.navy, color: C.white, padding: '18px 20px', display: 'flex', flexDirection: 'column', gap: 10, ...rise('vIn6', 0, 300) }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: C.onlineText }}><span style={{ width: 6, height: 6, borderRadius: 3, background: C.online }} />Licence active</span>
            <span style={{ fontFamily: SERIF, fontSize: 26, lineHeight: 1.1 }}>One-time licence, <span style={{ fontStyle: 'italic', color: C.gold }}>yours to keep.</span></span>
            <span style={{ fontSize: 13, color: C.onNavyMuted }}>Bound to this computer · No yearly fees · Unlimited students</span>
          </div>
          <Btn full h={50} icon="arrow" hl="cta">Continue to setup</Btn>
        </>
      )}
    </Split>
  );
}

const CLASSES = ['Nursery', 'LKG', 'UKG', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X'];
export function SetupScreen({ step = 'school' }) {
  const stepName = { school: 'School', session: 'Session & classes', you: 'You', recovery: 'Recovery key', ready: 'Ready' }[step];
  const n = SETUP_STEPS.indexOf(stepName) + 1;
  let body;
  if (step === 'school') body = (
    <>
      <TwoLine l1="About your school." l2="It appears on receipts." size={44} track="-0.03em" />
      <Field label="School name" value="Saraswati Public School" focused hl="name" />
      <Field label="Address" value="12-2-823, Mehdipatnam, Hyderabad" />
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        <Field label="Board" value="State Board" suffix={<Icon name="chevronDown" size={16} sw={1.75} color={C.muted} />} />
        <Field label="UDISE code" placeholder="Optional" />
      </div>
    </>
  );
  if (step === 'session') body = (
    <>
      <TwoLine l1="Session and classes." l2="Edit anything later." size={44} track="-0.03em" />
      <div data-hl="session" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', border: `1px solid ${C.line}`, borderRadius: 10, background: C.white, overflow: 'hidden' }}>
        {[['Session', '2026–27'], ['Term 1', 'Apr – Sep'], ['Term 2', 'Oct – Mar']].map(([a, b], i) => (
          <div key={a} style={{ padding: '12px 14px', borderLeft: i ? `1px solid ${C.line}` : 0, display: 'flex', flexDirection: 'column', gap: 4 }}><Label>{a}</Label><span style={{ fontFamily: SERIF, fontSize: 22 }}>{b}</span></div>
        ))}
      </div>
      <div data-hl="classes" style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        <span style={{ fontSize: 13, fontWeight: 500 }}>Classes · section A</span>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
          {CLASSES.map((c) => <Chip key={c} on>{c}</Chip>)}
          <Chip>+ Section</Chip>
        </div>
      </div>
      <div data-hl="grading" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '14px 16px', borderRadius: 8, border: `1px solid ${C.line}`, background: C.white }}>
        <span style={{ display: 'flex', flexDirection: 'column', gap: 3 }}><span style={{ fontWeight: 600 }}>Grading scale</span><span style={{ fontSize: 13, color: C.muted }}>Set by your school · start from a template</span></span>
        <Btn variant="secondary" h={36}>Choose</Btn>
      </div>
    </>
  );
  if (step === 'you') body = (
    <>
      <TwoLine l1="You, the Principal." l2="Your PIN unlocks Vidya." size={44} track="-0.03em" />
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        <Field label="Your name" value="Priya Sharma" />
        <Field label="Mobile" value="98480 21xxx" />
      </div>
      <div data-hl="pin" style={{ display: 'flex', flexDirection: 'column', gap: 12, padding: '18px 18px', borderRadius: 10, background: C.white, border: `1px solid ${C.line}` }}>
        <span style={{ fontSize: 13, fontWeight: 500 }}>Create a 6-digit PIN</span><PinDots filled={6} />
        <span style={{ fontSize: 13, fontWeight: 500, marginTop: 6 }}>Enter it again</span><PinDots filled={6} />
        <span style={{ fontSize: 12, color: C.muted }}>Used to unlock this computer. It never leaves it.</span>
      </div>
    </>
  );
  if (step === 'recovery') body = (
    <>
      <TwoLine l1="Your recovery key." l2="Keep it somewhere safe." size={44} track="-0.03em" />
      <p style={{ margin: 0, fontSize: 14, lineHeight: 1.55, color: C.muted }}>You need it to restore a backup or move Vidya to a new computer. Nobody, including us, can recover it for you.</p>
      <div data-hl="key" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 8, padding: 16, borderRadius: 10, background: C.navy }}>
        {['7KQ2M', '9XDT4', 'RB8WH', 'N3VCF', 'Q6PJZ', 'E1TKA'].map((g, i) => (
          <span key={g} style={{ height: 44, borderRadius: 6, background: 'rgba(255,255,255,0.05)', border: `1px solid ${i === 2 || i === 4 ? 'rgba(197,171,122,0.5)' : 'rgba(255,255,255,0.1)'}`, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8, color: C.white, fontFamily: "'Geist', monospace", fontSize: 17, letterSpacing: '0.12em' }}>
            <span style={{ fontSize: 10, color: C.onNavyMuted, letterSpacing: 0 }}>{i + 1}</span>{g}
          </span>
        ))}
      </div>
      <div style={{ display: 'flex', gap: 10 }}><Btn variant="secondary" h={36} icon="print">Print</Btn><Btn variant="secondary" h={36} icon="copy">Copy</Btn></div>
      <div data-hl="confirm" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        <Field label="Type group 3" value="RB8WH" mono h={44} />
        <Field label="Type group 5" value="Q6PJZ" mono h={44} focused />
      </div>
    </>
  );
  if (step === 'ready') body = (
    <>
      <TwoLine l1="Your school is ready." l2="Invite your staff next." size={44} track="-0.03em" />
      <div data-hl="checklist" style={{ display: 'flex', flexDirection: 'column', border: `1px solid ${C.line}`, borderRadius: 10, background: C.white, overflow: 'hidden' }}>
        {[['Licence active', true], ['School created · 13 classes', true], ['Recovery key saved', true], ['Invite teachers and accountant', false], ['Connect Google Drive for backups', false]].map(([t, d], i) => (
          <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '13px 16px', borderTop: i ? `1px solid ${C.track}` : 0, ...rise('vIn6', i * 90, 300) }}>
            <span style={{ width: 22, height: 22, borderRadius: 11, display: 'flex', alignItems: 'center', justifyContent: 'center', background: d ? C.accent : 'transparent', border: d ? 0 : `1.5px solid ${C.radioOff}`, color: C.white }}>{d && <Icon name="check" size={13} sw={2.4} />}</span>
            <span style={{ fontSize: 14, color: d ? C.ink : C.muted }}>{t}</span>
            {!d && <span style={{ marginLeft: 'auto', fontSize: 12, color: C.goldText, fontWeight: 600 }}>Next</span>}
          </div>
        ))}
      </div>
    </>
  );
  return (
    <Split right={<StepsPanel current={stepName} />}>
      <Eyebrow>Step {n} of 6</Eyebrow>
      {body}
      <div style={{ display: 'flex', gap: 10 }}>
        {step !== 'ready' && <Btn variant="secondary" h={50}>Back</Btn>}
        <div style={{ flexGrow: 1 }}><Btn full h={50} icon="arrow" hl="cta">{step === 'ready' ? 'Open Vidya' : 'Continue'}</Btn></div>
      </div>
    </Split>
  );
}

export function PinScreen({ filled = 4 }) {
  return (
    <Split right={<BrandPanel />}>
      <Eyebrow>Saraswati Public School</Eyebrow>
      <TwoLine l1="Welcome back." l2="Enter your PIN." size={56} track="-0.03em" />
      <div style={{ display: 'flex', gap: 10 }}>
        {[['PS', 'Priya Sharma', 'Principal', true], ['SP', 'Suresh Patel', 'Accountant', false]].map(([i, n, r, on]) => (
          <div key={n} style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 10, padding: '12px 14px', borderRadius: 8, border: `1.5px solid ${on ? C.accent : C.line}`, background: on ? acc(0.06) : C.white }}>
            <span style={{ width: 34, height: 34, borderRadius: 17, background: C.navy, color: C.goldLight, display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 13, fontWeight: 600 }}>{i}</span>
            <span style={{ display: 'flex', flexDirection: 'column' }}><span style={{ fontWeight: 600 }}>{n}</span><span style={{ fontSize: 12, color: C.muted }}>{r}</span></span>
          </div>
        ))}
      </div>
      <div data-hl="pin" style={{ padding: '22px 20px', borderRadius: 10, background: C.white, border: `1px solid ${C.line}`, display: 'flex', justifyContent: 'center' }}><PinDots filled={filled} /></div>
      <Btn full h={50} icon="arrow" disabled={filled < 6}>Unlock</Btn>
    </Split>
  );
}

/* ================================================================== SCREENS: principal */
const APPROVALS = [
  ['marks', 'Marks correction', 'Half Yearly · VI-B Maths · Kavya Singh 62 → 72', 'Anita Rao, Teacher', '2 hours ago'],
  ['reversal', 'Payment reversal', 'Receipt R-A2-0418 · ₹1,500 · entered twice', 'Suresh Patel, Accountant', '5 hours ago'],
  ['attendance', 'Attendance correction', 'V-A · 19 Sep · Rahul Kumar: Absent → Present', 'Meena Iyer, Teacher', '10 minutes ago'],
  ['details', 'Student details', 'Aarav Gupta · father’s mobile number', 'Suresh Patel, Accountant', '2 days ago'],
];
export function PrincipalHome() {
  const stats = [['91.4%', 'attendance today', '612 of 670 marked · VII-B pending'], ['₹48,500', 'collected today', '23 receipts · ₹2,400 more waiting for server'], ['₹6,84,200', 'outstanding this term', '212 students with dues'], ['670', 'active students', '4 new admissions this week']];
  const cls = [['Nursery', 96], ['LKG', 92], ['I-A', 94], ['II-A', 90], ['III-A', 93], ['V-A', 91], ['VII-B', null]];
  const days = [['Thu', 32000], ['Fri', 41000], ['Sat', 28500], ['Mon', 55000], ['Tue', 46200], ['Today', 48500]];
  return (
    <Desktop role="principal" active="home">
      <main style={{ flexGrow: 1, padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: 24, minHeight: 0 }}>
        <div data-hl="greeting"><PageTitle eyebrow="Wednesday, 23 September · Term 1" l1="Good morning, Priya." l2="Four approvals are waiting."
          actions={<><Btn variant="secondary" icon="plus">New admission</Btn><Btn icon="arrow">Review approvals</Btn></>} /></div>
        <div data-hl="stats" style={{ display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0,1fr))', background: C.panel, borderRadius: 16, ...rise('vRise12', 80) }}>
          {stats.map(([v, l, n], i) => (
            <a key={l} href="#" style={{ display: 'flex', flexDirection: 'column', gap: 10, padding: '22px 24px', textDecoration: 'none', color: 'inherit', borderLeft: i ? `1px solid ${C.lineStat}` : 0 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}><span style={{ fontFamily: SERIF, fontSize: 40, lineHeight: 1, letterSpacing: '-0.02em', fontVariantNumeric: 'lining-nums tabular-nums' }}>{v}</span><Icon name="arrow" size={14} sw={1.75} /></div>
              <span style={{ fontSize: 14 }}>{l}</span><span style={{ fontSize: 12, color: C.muted }}>{n}</span>
            </a>
          ))}
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 24, ...rise('vRise12', 160) }}>
          <Card eyebrow="Approvals" title="Waiting for your decision" link="Open all" hl="approvals">
            {APPROVALS.map(([tone, t, w, who, age]) => (
              <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '14px 24px', borderTop: `1px solid ${C.track}` }}>
                <Pill tone={tone} fixed>{t}</Pill>
                <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: 3, minWidth: 0 }}><span style={{ fontWeight: 500, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{w}</span><span style={{ fontSize: 12, color: C.muted }}>{who} · {age}</span></div>
                <Btn variant="secondary" h={34} style={{ fontSize: 13 }}>Review</Btn>
              </div>
            ))}
          </Card>
          <Card eyebrow="Today" title="Needs attention" hl="attention" footer={
            <div style={{ background: C.panel, padding: '14px 24px', display: 'flex', flexDirection: 'column', gap: 3 }}><Label>Last backup</Label><span style={{ fontSize: 13 }}>Verified today at 6:02 AM · this PC and school Drive</span></div>}>
            <div style={{ flexGrow: 1 }}>
              {[[C.gold, 'VII-B attendance not submitted', 'Class teacher R. Nair · not submitted yet today'], [C.danger, 'One edit conflict to review', 'Riya Verma · address changed on two phones']].map(([c, t, s]) => (
                <div key={t} style={{ display: 'flex', gap: 12, padding: '14px 24px', borderTop: `1px solid ${C.track}` }}><span style={{ width: 8, height: 8, marginTop: 6, borderRadius: 4, background: c, flexShrink: 0 }} /><div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}><span style={{ fontWeight: 500 }}>{t}</span><span style={{ fontSize: 12, color: C.muted }}>{s}</span></div></div>
              ))}
            </div>
          </Card>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '3fr 2fr', gap: 24, minHeight: 0, ...rise('vRise12', 240) }}>
          <section data-hl="byclass" style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, padding: '18px 24px', display: 'flex', flexDirection: 'column', gap: 10 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}><h2 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 22 }}>Attendance by class</h2><a href="#" style={{ fontSize: 13, textDecoration: 'none', color: C.accent }}>Full register</a></div>
            {cls.map(([n, p], i) => (
              <div key={n} style={{ display: 'flex', alignItems: 'center', gap: 14, height: 24 }}>
                <span style={{ width: 60, fontSize: 13 }}>{n}</span>
                <div style={{ flexGrow: 1, height: 6, borderRadius: 3, background: C.track, overflow: 'hidden' }}><div style={{ height: '100%', borderRadius: 3, transformOrigin: 'left', width: `${p || 0}%`, background: C.accent, animation: `vFill 800ms ${300 + i * 60}ms ${EASE} both` }} /></div>
                <span style={{ width: 110, textAlign: 'right', fontSize: 12, fontVariantNumeric: 'tabular-nums', color: p == null ? C.goldText : C.muted, fontWeight: p == null ? 600 : 400 }}>{p == null ? 'Not submitted' : `${p}% present`}</span>
              </div>
            ))}
          </section>
          <section data-hl="feechart" style={{ background: C.navy, color: C.white, borderRadius: 16, padding: '18px 24px', display: 'flex', flexDirection: 'column', gap: 10 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}><h2 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 22 }}>Fee collection</h2><span style={{ fontSize: 12, color: C.onNavyMuted }}>Last 6 school days</span></div>
            <span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 28, color: C.gold }}>₹2,51,200</span>
            <div style={{ flexGrow: 1, display: 'flex', alignItems: 'flex-end', gap: 14, minHeight: 110 }}>
              {days.map(([d, v], i) => (
                <div key={d} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6 }}>
                  <div style={{ width: '100%', maxWidth: 36, borderRadius: '4px 4px 2px 2px', transformOrigin: 'bottom', height: Math.round(v / 55000 * 96), background: d === 'Today' ? C.gold : 'rgba(125,177,181,0.55)', animation: `vGrow 700ms ${300 + i * 70}ms ${EASE} both` }} />
                  <span style={{ fontSize: 11, color: C.onNavyMuted }}>{d}</span>
                </div>
              ))}
            </div>
          </section>
        </div>
      </main>
    </Desktop>
  );
}

const STAFF = [
  ['PS', 'Priya Sharma', 'Principal', 'Everything', 'Active'],
  ['SP', 'Suresh Patel', 'Accountant', 'Admissions, fees, receipts', 'Active'],
  ['AR', 'Anita Rao', 'Teacher', 'Maths · VI-B, VII-A', 'Active'],
  ['RN', 'R. Nair', 'Teacher', 'Class teacher VII-B · English', 'Active'],
];
export function StaffScreen({ panel = 'none' }) {
  const rows = panel === 'invite' ? [...STAFF, ['MI', 'Meena Iyer', 'Teacher', 'Class teacher V-A · Maths V-A', 'Invited']] : STAFF;
  return (
    <Desktop role="principal" active="staff">
      <main style={{ flexGrow: 1, padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: 24 }}>
        <PageTitle eyebrow="School · 5 staff" l1="Staff & access." l2="Everyone sees only their work." actions={<Btn icon="plus" hl="addstaff">Add staff</Btn>} />
        <div style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, overflow: 'hidden' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 2.2fr 1fr', padding: '12px 24px', borderBottom: `1px solid ${C.track}` }}>{['Staff', 'Role', 'Access', 'Status'].map((h) => <Label key={h}>{h}</Label>)}</div>
          {rows.map(([i, n, r, a, s]) => (
            <div key={n} data-hl={n === 'Meena Iyer' ? 'newrow' : undefined} style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 2.2fr 1fr', alignItems: 'center', padding: '14px 24px', borderBottom: `1px solid ${C.track}`, ...(n === 'Meena Iyer' ? rise('vIn8', 0, 300) : {}) }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: 12 }}><span style={{ width: 34, height: 34, borderRadius: 17, background: C.navy, color: C.goldLight, display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 13, fontWeight: 600 }}>{i}</span><span style={{ fontWeight: 500 }}>{n}</span></span>
              <span>{r}</span><span style={{ color: C.muted }}>{a}</span>
              <Pill tone={s === 'Active' ? 'paid' : 'partpaid'}>{s === 'Invited' ? 'Invite sent' : s}</Pill>
            </div>
          ))}
        </div>
      </main>
      {panel !== 'none' && <Overlay />}
      {panel === 'add' && (
        <SheetPanel eyebrow="Add staff" title="Meena Iyer" sub="Invite a teacher, accountant or another principal">
          <Field label="Name" value="Meena Iyer" h={46} />
          <Field label="Mobile" value="97010 55xxx" h={46} />
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><span style={{ fontSize: 13, fontWeight: 500 }}>Role</span><Segmented options={['Teacher', 'Accountant']} value="Teacher" hl="role" /></div>
          <Field label="Class teacher of" value="V-A · 34 students" h={46} hl="classteacher" suffix={<Icon name="chevronDown" size={16} sw={1.75} color={C.muted} />} />
          <div data-hl="subjects" style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><span style={{ fontSize: 13, fontWeight: 500 }}>Subjects she teaches</span><div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}><Chip on>Maths · V-A</Chip><Chip>+ Add subject</Chip></div></div>
          <div data-hl="preview" style={{ borderRadius: 10, background: C.panel, padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 8 }}>
            <Label>What Meena will be able to do</Label>
            {['Take attendance for V-A', 'Enter marks for V-A Maths', 'See V-A students (no fee details)', 'Send correction requests to you'].map((t) => (
              <span key={t} style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13 }}><Icon name="check" size={14} sw={2.2} color={C.accent} />{t}</span>
            ))}
          </div>
          <div style={{ marginTop: 'auto' }}><Btn full h={52} icon="arrow" hl="create">Create invitation</Btn></div>
        </SheetPanel>
      )}
      {panel === 'invite' && (
        <SheetPanel eyebrow="Invitation ready" title="Meena Iyer" sub="Teacher · Class teacher V-A">
          <div data-hl="qr" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14, padding: '22px 16px', borderRadius: 12, background: C.white, border: `1px solid ${C.line}`, ...rise('vIn8', 0, 300) }}>
            <FakeQR />
            <span style={{ fontSize: 13, color: C.muted }}>Scan with the Vidya app, or type the code</span>
            <span data-hl="invitecode" style={{ fontFamily: SERIF, fontSize: 40, letterSpacing: '0.08em' }}>K7M2-QX9D</span>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
            <div style={{ borderRadius: 10, background: C.panel, padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 4 }}><Label>Check code</Label><span style={{ fontFamily: "'Geist', monospace", fontSize: 17, letterSpacing: '0.1em' }}>3F9A 1C7E</span></div>
            <div style={{ borderRadius: 10, background: C.panel, padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 4 }}><Label>Expires</Label><span style={{ fontSize: 15 }}>In 72 hours · single use</span></div>
          </div>
          <div style={{ marginTop: 'auto', display: 'flex', flexDirection: 'column', gap: 10 }}>
            <Btn full h={52} icon="arrow" hl="share">Share on WhatsApp</Btn>
            <Btn full h={46} variant="secondary" icon="copy">Copy link</Btn>
          </div>
        </SheetPanel>
      )}
    </Desktop>
  );
}
function Overlay() { return <div style={{ position: 'absolute', inset: 0, background: 'rgba(11,26,51,0.45)', animation: 'vFade 240ms ease both' }} />; }
function SheetPanel({ eyebrow, title, sub, children }) {
  return (
    <section data-hl="sheet" style={{ position: 'absolute', top: 16, right: 16, bottom: 16, width: 520, background: C.surface, borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', display: 'flex', flexDirection: 'column', overflow: 'hidden', animation: `vSheet 380ms ${EASE} both` }}>
      <div style={{ padding: '24px 28px 18px', display: 'flex', flexDirection: 'column', gap: 14, borderBottom: `1px solid ${C.track}` }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}><Eyebrow>{eyebrow}</Eyebrow>
          <button type="button" aria-label="Close" style={{ width: 36, height: 36, borderRadius: 18, border: `1px solid ${C.line}`, background: C.white, color: C.muted, display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Icon name="close" size={16} sw={1.75} /></button></div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}><span style={{ fontFamily: SERIF, fontSize: 28, lineHeight: 1.1, letterSpacing: '-0.015em' }}>{title}</span><span style={{ fontSize: 13, color: C.muted }}>{sub}</span></div>
      </div>
      <div style={{ flexGrow: 1, overflow: 'auto', padding: '20px 28px 24px', display: 'flex', flexDirection: 'column', gap: 18 }}>{children}</div>
    </section>
  );
}

export function ApprovalsScreen({ state = 'review' }) {
  if (state === 'leave' || state === 'leaveApproved') return <LeaveApproval done={state === 'leaveApproved'} />;
  const decided = state === 'approved';
  const list = decided ? APPROVALS.filter((a) => a[0] !== 'attendance') : APPROVALS;
  return (
    <Desktop role="principal" active="approvals" badge={decided ? 3 : 4}>
      <main style={{ flexGrow: 1, padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: 22, minHeight: 0 }}>
        <PageTitle eyebrow={`${list.length} waiting · oldest 2 days`} l1="Approvals." l2="Check, then decide." />
        <div style={{ width: 720 }}><Segmented options={['All', 'Marks', 'Payment', 'Attendance', 'Student details']} value="All" /></div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 520px', gap: 24, minHeight: 0, flexGrow: 1 }}>
          <Card>
            {list.map(([tone, t, w, who, age]) => {
              const sel = tone === 'attendance';
              return (
                <div key={t} data-hl={sel ? 'row' : undefined} style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '16px 24px', borderTop: `1px solid ${C.track}`, background: sel ? acc(0.06) : 'transparent', boxShadow: sel ? `inset 3px 0 0 ${C.accent}` : 'none' }}>
                  <Pill tone={tone} fixed>{t}</Pill>
                  <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: 3, minWidth: 0 }}><span style={{ fontWeight: 500, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{w}</span><span style={{ fontSize: 12, color: C.muted }}>{who} · {age}</span></div>
                </div>
              );
            })}
          </Card>
          {!decided ? (
            <section data-hl="detail" style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, display: 'flex', flexDirection: 'column', overflow: 'hidden', ...rise('vIn8', 0, 300) }}>
              <div style={{ padding: '22px 24px 16px', display: 'flex', flexDirection: 'column', gap: 12, borderBottom: `1px solid ${C.track}` }}>
                <Pill tone="attendance" fixed>Attendance correction</Pill>
                <span style={{ fontFamily: SERIF, fontSize: 28, lineHeight: 1.1 }}>Rahul Kumar · V-A</span>
                <span style={{ fontSize: 13, color: C.muted }}>Requested by Meena Iyer, Teacher · 10 minutes ago</span>
              </div>
              <div style={{ padding: '18px 24px', display: 'flex', flexDirection: 'column', gap: 16, flexGrow: 1 }}>
                <div data-hl="beforeafter" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', border: `1px solid ${C.line}`, borderRadius: 10, overflow: 'hidden' }}>
                  {[['Date', '19 Sep 2026', null], ['Before', 'Absent', C.danger], ['After', 'Present', C.accent]].map(([a, b, c], i) => (
                    <div key={a} style={{ padding: '12px 14px', borderLeft: i ? `1px solid ${C.line}` : 0, background: i === 2 ? acc(0.06) : C.white, display: 'flex', flexDirection: 'column', gap: 4 }}><Label>{a}</Label><span style={{ fontFamily: SERIF, fontSize: 22, color: c || C.ink }}>{b}</span></div>
                  ))}
                </div>
                <div data-hl="reason" style={{ display: 'flex', flexDirection: 'column', gap: 6 }}><Label>Reason</Label><span style={{ fontSize: 15, lineHeight: 1.5 }}>“He came late after a doctor’s visit; I marked him absent by mistake. The note from his parents is with me.”</span></div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}><Label>History</Label><span style={{ fontSize: 13, color: C.muted }}>Sheet submitted 19 Sep, 9:12 AM by Meena Iyer · first request for this record</span></div>
              </div>
              <div style={{ background: C.panel, padding: '16px 24px 20px', display: 'flex', gap: 10 }}>
                <Btn variant="secondary" h={50}>Return</Btn><Btn variant="secondary" h={50}>Reject</Btn>
                <div style={{ flexGrow: 1 }}><Btn full h={50} icon="check" hl="approve">Approve</Btn></div>
              </div>
            </section>
          ) : (
            <section data-hl="done" style={{ borderRadius: 16, background: GRAD.success, color: C.white, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 16, padding: 40, textAlign: 'center' }}>
              <div style={{ width: 76, height: 76, borderRadius: 38, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center', animation: `vPopFee 460ms ${EASE} both` }}><Icon name="check" size={34} sw={1.8} /></div>
              <div style={{ ...rise('vIn8', 120, 320) }}><TwoLine l1="Approved." l2="Register updated." size={40} color={C.white} italicColor={C.gold} track="-0.02em" as="span" /></div>
              <span style={{ fontSize: 14, color: C.onNavy, ...rise('vIn8', 200, 320) }}>Rahul Kumar · 19 Sep · now Present · Meena Iyer has been notified</span>
              <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: C.onlineText, border: '1px solid rgba(95,208,160,0.35)', borderRadius: 16, padding: '5px 12px', ...rise('vIn8', 280, 320) }}><span style={{ width: 6, height: 6, borderRadius: 3, background: C.online }} />Recorded in the audit history</span>
            </section>
          )}
        </div>
      </main>
    </Desktop>
  );
}

/* ================================================================== SCREENS: accountant */
export function CollectFeeScreen({ amount = '1000', mode = 'UPI', done = false }) {
  const due = 3100; const amt = parseInt(amount, 10) || 0; const over = amt > due;
  const fmt = (v) => '₹' + v.toLocaleString('en-IN');
  const cantSave = amt <= 0 || over;
  const refs = { UPI: ['UPI transaction ID', 'e.g. 426518903214'], Cheque: ['Cheque number and bank', 'e.g. 004512 · SBI'] };
  const rows = [['Kavya Singh', '2023/0287', 'VI-B', '₹3,100', 'Part paid', 'partpaid'], ['Kavya Mishra', '2025/0142', 'II-A', '₹0', 'Paid', 'paid'], ['Kavya Reddy', '2024/0519', 'IV-A', '₹6,500', 'Unpaid', 'unpaid']];
  return (
    <Desktop role="accountant" active="fees" header={false}>
      <main style={{ padding: 48, display: 'flex', flexDirection: 'column', gap: 28 }}>
        <PageTitle eyebrow="Fees · Term 2" l1="Collect a fee." l2="Find the student first." />
        <div data-hl="search" style={{ position: 'relative', width: 560 }}>
          <Icon name="search" size={18} sw={1.75} color={C.muted} style={{ position: 'absolute', left: 14, top: 14 }} />
          <div style={{ height: 46, borderRadius: 6, border: `1px solid ${C.lineStrong}`, background: C.surface, padding: '0 16px 0 42px', display: 'flex', alignItems: 'center', fontSize: 15 }}>kavya</div>
        </div>
        <div style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, overflow: 'hidden' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '2.2fr 1fr 1fr 1fr', padding: '12px 24px', borderBottom: `1px solid ${C.track}` }}>{['Student', 'Class', 'Due', 'Status'].map((h) => <Label key={h}>{h}</Label>)}</div>
          {rows.map(([n, adm, c, d, s, tone]) => (
            <div key={n} style={{ display: 'grid', gridTemplateColumns: '2.2fr 1fr 1fr 1fr', alignItems: 'center', padding: '14px 24px', borderBottom: `1px solid ${C.track}` }}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}><span style={{ fontWeight: 500 }}>{n}</span><span style={{ fontSize: 12, color: C.muted }}>Adm. {adm}</span></div>
              <span>{c}</span><span style={{ fontVariantNumeric: 'tabular-nums' }}>{d}</span><Pill tone={tone}>{s}</Pill>
            </div>
          ))}
        </div>
      </main>
      <Overlay />
      <section data-hl="sheet" style={{ position: 'absolute', top: 16, right: 16, bottom: 16, width: 520, background: C.surface, borderRadius: 20, boxShadow: '0 30px 60px rgba(11,26,51,0.3)', display: 'flex', flexDirection: 'column', overflow: 'hidden', animation: `vSheet 380ms ${EASE} both` }}>
        {!done ? (
          <>
            <div style={{ padding: '24px 28px 18px', display: 'flex', flexDirection: 'column', gap: 14, borderBottom: `1px solid ${C.track}` }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}><Eyebrow>Record a payment</Eyebrow>
                <button type="button" aria-label="Close" style={{ width: 36, height: 36, borderRadius: 18, border: `1px solid ${C.line}`, background: C.white, color: C.muted, display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Icon name="close" size={16} sw={1.75} /></button></div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
                <div style={{ width: 48, height: 48, borderRadius: 24, background: C.navy, color: C.goldLight, display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: SERIF, fontSize: 20 }}>KS</div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}><span style={{ fontFamily: SERIF, fontSize: 28, lineHeight: 1.1, letterSpacing: '-0.015em' }}>Kavya Singh</span><span style={{ fontSize: 13, color: C.muted }}>Class VI-B · Roll 14 · Adm. 2023/0287 · Transport</span></div>
              </div>
            </div>
            <div style={{ flexGrow: 1, overflow: 'auto', padding: '20px 28px', display: 'flex', flexDirection: 'column', gap: 20 }}>
              <div data-hl="table" style={{ display: 'flex', flexDirection: 'column' }}>
                {[['Term 1 tuition', '₹6,000', 'Paid'], ['Term 2 tuition', '₹6,000', '₹2,400 due'], ['Exam fee', '₹500', '₹500 due'], ['Transport · September', '₹200', '₹200 due']].map(([a, b, c]) => (
                  <div key={a} style={{ display: 'grid', gridTemplateColumns: '1fr 80px 96px', padding: '8px 0', fontSize: 13, borderBottom: `1px solid ${C.track}` }}><span>{a}</span><span style={{ textAlign: 'right', color: C.muted, fontVariantNumeric: 'tabular-nums' }}>{b}</span><span style={{ textAlign: 'right', fontWeight: 500, color: c === 'Paid' ? C.accent : C.ink, fontVariantNumeric: 'tabular-nums' }}>{c}</span></div>
                ))}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', paddingTop: 12 }}><span style={{ fontWeight: 500 }}>Total due now</span><span style={{ fontFamily: SERIF, fontSize: 24, fontVariantNumeric: 'lining-nums tabular-nums' }}>₹3,100</span></div>
              </div>
              <div data-hl="amount" style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                <span style={{ fontSize: 13, fontWeight: 500 }}>Amount received</span>
                <div style={{ display: 'flex', alignItems: 'center', height: 62, borderRadius: 6, background: C.white, border: `1.5px solid ${over ? C.danger : C.accent}`, boxShadow: `0 0 0 4px ${over ? 'rgba(192,57,43,0.14)' : acc(0.12)}` }}>
                  <span style={{ fontFamily: SERIF, fontSize: 28, color: C.muted, paddingLeft: 16 }}>₹</span>
                  <span style={{ fontFamily: SERIF, fontSize: 30, paddingLeft: 6, fontVariantNumeric: 'lining-nums tabular-nums' }}>{amount}</span>
                </div>
                {over && <span style={{ fontSize: 13, color: C.unFg, ...rise('vIn8', 0, 200) }}>That is more than the ₹3,100 due. Please check the amount.</span>}
                <div style={{ display: 'flex', gap: 8 }}><Chip on={amt === 3100}>Full due · ₹3,100</Chip><Chip on={amt === 1000} hl="chip1000">₹1,000</Chip><Chip on={amt === 500}>₹500</Chip></div>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><span style={{ fontSize: 13, fontWeight: 500 }}>Payment mode</span><Segmented options={['Cash', 'UPI', 'Cheque']} value={mode} hl="modes" /></div>
              {mode !== 'Cash' && <Field label={refs[mode][0]} placeholder={refs[mode][1]} h={46} hl="ref" />}
            </div>
            <div data-hl="balance" style={{ background: C.panel, padding: '16px 28px 20px', display: 'flex', flexDirection: 'column', gap: 14 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}><Label>Balance after</Label><span style={{ fontSize: 12, color: C.muted }}>₹3,100 due − {fmt(amt)} this payment</span></div>
                <span style={{ fontFamily: SERIF, fontSize: 30, fontVariantNumeric: 'lining-nums tabular-nums' }}>{fmt(Math.max(due - amt, 0))}</span>
              </div>
              <Btn full h={52} icon="arrow" disabled={cantSave} hl="record">{cantSave ? 'Enter a valid amount' : `Record ${fmt(amt)} payment`}</Btn>
              <span style={{ fontSize: 12, color: C.muted, textAlign: 'center' }}>A receipt number is given when you record the payment.</span>
            </div>
          </>
        ) : (
          <div data-hl="success" style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 18, padding: 40, textAlign: 'center', background: GRAD.success, color: C.white }}>
            <div style={{ width: 76, height: 76, borderRadius: 38, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center', animation: `vPopFee 460ms ${EASE} both` }}><Icon name="check" size={34} sw={1.8} /></div>
            <div style={rise('vIn8', 120, 320)}><TwoLine l1="Payment recorded." l2="Receipt R-A2-0419." size={40} color={C.white} italicColor={C.gold} track="-0.02em" as="span" /></div>
            <span style={{ fontSize: 14, color: C.onNavy, ...rise('vIn8', 200, 320) }}>{fmt(amt)} by {mode} · Kavya Singh · {fmt(Math.max(due - amt, 0))} still due</span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: C.onlineText, border: '1px solid rgba(95,208,160,0.35)', borderRadius: 16, padding: '5px 12px', ...rise('vIn8', 280, 320) }}><span style={{ width: 6, height: 6, borderRadius: 3, background: C.online }} />Confirmed by school server</span>
            <div data-hl="buttons" style={{ display: 'flex', gap: 10, marginTop: 10, ...rise('vIn8', 360, 320) }}><Btn variant="gold" h={46} hl="print">Print receipt</Btn><Btn variant="ghostNavy" h={46}>Share on WhatsApp</Btn></div>
            <button type="button" style={{ border: 0, background: 'transparent', color: C.onNavy, fontSize: 14, height: 40, textDecoration: 'underline', textUnderlineOffset: 4 }}>Collect another fee</button>
          </div>
        )}
      </section>
    </Desktop>
  );
}

export function ReceiptScreen() {
  const row = (a, b, strong) => (
    <div style={{ display: 'flex', justifyContent: 'space-between', padding: '7px 0', borderBottom: `1px solid ${C.track}`, fontSize: 13 }}><span style={{ color: C.muted }}>{a}</span><span style={{ fontWeight: strong ? 600 : 400, fontVariantNumeric: 'tabular-nums' }}>{b}</span></div>
  );
  return (
    <div className="vd" style={{ width: 1440, height: 1080, background: C.outer, fontFamily: SANS, color: C.ink, display: 'flex', flexDirection: 'column' }}>
      <div style={{ height: 64, background: C.navy, color: C.white, display: 'flex', alignItems: 'center', padding: '0 32px', gap: 16 }}>
        <Icon name="print" size={20} color={C.gold} />
        <span style={{ fontFamily: SERIF, fontSize: 22 }}>Print receipt</span><span style={{ color: C.onNavyMuted, fontSize: 13 }}>A5 · 1 page · Fee receipt R-A2-0419</span>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 10 }}><Btn variant="ghostNavy" h={40}>Cancel</Btn><Btn variant="gold" h={40} hl="printbtn">Print</Btn></div>
      </div>
      <div style={{ flexGrow: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <div data-hl="paper" style={{ width: 560, background: C.white, boxShadow: '0 30px 60px rgba(11,26,51,0.18)', padding: '36px 40px', display: 'flex', flexDirection: 'column', gap: 14, ...rise('vIn8', 0, 380) }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
            <img src={A.logoLight} alt="Vidya Budget School" style={{ width: 150, height: 49 }} />
            <div style={{ textAlign: 'right', display: 'flex', flexDirection: 'column', gap: 2 }}><span style={{ fontFamily: SERIF, fontSize: 20 }}>Saraswati Public School</span><span style={{ fontSize: 12, color: C.muted }}>12-2-823, Mehdipatnam, Hyderabad</span></div>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', borderTop: `2px solid ${C.navy}`, paddingTop: 14 }}>
            <span style={{ fontFamily: SERIF, fontSize: 30 }}>Fee receipt</span><span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 22, color: C.accent }}>R-A2-0419</span>
          </div>
          <div>{row('Date', '23 Sep 2026, 11:42 AM')}{row('Student', 'Kavya Singh')}{row('Class · Adm. no.', 'VI-B · 2023/0287')}</div>
          <div data-hl="heads" style={{ display: 'flex', flexDirection: 'column' }}>
            <Label>Paid towards</Label>
            {row('Term 2 tuition (part)', '₹1,000')}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', paddingTop: 10 }}><span style={{ fontWeight: 600 }}>Total received</span><span style={{ fontFamily: SERIF, fontSize: 28 }}>₹1,000</span></div>
            <span style={{ fontSize: 12, color: C.muted, fontStyle: 'italic' }}>One thousand rupees only</span>
          </div>
          <div>{row('Mode', 'UPI · ref •••• 3214')}{row('Balance after', '₹2,100', true)}{row('Collected by', 'Suresh Patel')}</div>
          <div data-hl="upiqr" style={{ display: 'flex', alignItems: 'center', gap: 16, padding: 12, borderRadius: 8, background: C.bg, border: `1px dashed ${C.lineStrong}` }}>
            <FakeQR size={84} seed={11} />
            <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}><span style={{ fontWeight: 600, fontSize: 13 }}>Pay the balance by UPI</span><span style={{ fontSize: 12, color: C.muted }}>Scan to pay ₹2,100 to saraswatischool@okhdfcbank. Amount and student name are filled in.</span></div>
          </div>
          <span style={{ fontSize: 11, color: C.muted, textAlign: 'center', marginTop: 6 }}>Confirmed by school server · Computer-generated receipt</span>
        </div>
      </div>
    </div>
  );
}

/* ================================================================== SCREENS: phone */
function Phone({ children, bg = C.bg, dark }) {
  return <div className="vd" style={{ position: 'relative', width: 390, height: 844, display: 'flex', flexDirection: 'column', background: bg, color: dark ? C.white : C.ink, fontFamily: SANS, fontSize: 14, overflow: 'hidden' }}>{children}</div>;
}
function NetPill({ children, hl }) {
  return <span data-hl={hl} style={{ height: 30, display: 'flex', alignItems: 'center', gap: 6, padding: '0 12px', borderRadius: 15, fontSize: 12, fontWeight: 500, background: 'rgba(197,171,122,0.14)', color: C.goldOnNavy, border: '1px solid rgba(197,171,122,0.3)' }}><Icon name="cloudOff" size={13} sw={2} />{children}</span>;
}
function OnlinePill({ children }) {
  return <span style={{ height: 30, display: 'flex', alignItems: 'center', gap: 6, padding: '0 12px', borderRadius: 15, fontSize: 12, fontWeight: 500, background: 'rgba(95,208,160,0.12)', color: C.onlineText, border: '1px solid rgba(95,208,160,0.3)' }}><span style={{ width: 6, height: 6, borderRadius: 3, background: C.online }} />{children}</span>;
}
function PhoneHeader({ title, sub, pill, children }) {
  return (
    <header style={{ flexShrink: 0, background: GRAD.header, color: C.white, padding: '10px 16px 16px', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <a href="#" aria-label="Back" style={{ width: 44, height: 44, marginLeft: -10, borderRadius: 22, display: 'flex', alignItems: 'center', justifyContent: 'center', color: C.white }}><Icon name="back" size={22} sw={1.7} /></a>
        {pill}
      </div>
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        <h1 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 32, lineHeight: 1.05, letterSpacing: '-0.02em' }}>{title}</h1>
        <span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 22, lineHeight: 1.25, color: C.gold }}>{sub}</span>
      </div>
      {children}
    </header>
  );
}
function Counts({ items, hl }) {
  return (
    <div data-hl={hl} style={{ display: 'grid', gridTemplateColumns: `repeat(${items.length}, minmax(0,1fr))`, borderTop: '1px solid rgba(255,255,255,0.14)', paddingTop: 12 }}>
      {items.map(([v, l, gold], i) => (
        <div key={l} style={{ display: 'flex', flexDirection: 'column', gap: 2, borderLeft: i ? '1px solid rgba(255,255,255,0.12)' : 0, paddingLeft: i ? 12 : 0 }}>
          <span style={{ fontFamily: SERIF, fontSize: 26, lineHeight: 1, color: gold ? C.gold : C.white, fontVariantNumeric: 'lining-nums tabular-nums' }}>{v}</span><span style={{ fontSize: 11, color: C.onNavyMuted }}>{l}</span>
        </div>
      ))}
    </div>
  );
}
function PhoneLogoTop() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
      <img src={A.logoDark} alt="Vidya Budget School" style={{ width: 132, height: 43, display: 'block' }} />
      <div style={{ display: 'flex', gap: 8 }}>
        <button type="button" aria-label="Notifications" style={{ position: 'relative', width: 44, height: 44, borderRadius: 22, border: '1px solid rgba(255,255,255,0.14)', background: 'rgba(255,255,255,0.04)', color: C.white, display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Icon name="bell" size={19} /><span style={{ position: 'absolute', top: 10, right: 11, width: 7, height: 7, borderRadius: 4, background: C.gold }} /></button>
        <button type="button" aria-label="Account" style={{ width: 44, height: 44, borderRadius: 22, border: 0, background: C.navyRaised, color: C.goldLight, fontWeight: 600, fontSize: 14 }}>MI</button>
      </div>
    </div>
  );
}

export function JoinScreen({ step = 'code' }) {
  if (step === 'syncing') return (
    <Phone bg={GRAD.teacher} dark>
      <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 22, padding: 32, textAlign: 'center' }}>
        <img src={A.markDark} alt="" style={{ width: 120, height: 120, ...rise('vRise14', 0, 520) }} />
        <TwoLine l1="Welcome, Meena." l2="Getting your classes ready." size={30} color={C.white} italicColor={C.gold} track="-0.02em" as="span" />
        <div data-hl="progress" style={{ width: '100%', display: 'flex', flexDirection: 'column', gap: 10 }}>
          <div style={{ height: 6, borderRadius: 3, background: 'rgba(255,255,255,0.12)', overflow: 'hidden' }}><div style={{ width: '72%', height: '100%', background: C.gold, borderRadius: 3, transformOrigin: 'left', animation: `vFill 900ms ${EASE} both` }} /></div>
          <span style={{ fontSize: 13, color: C.onNavy }}>V-A · 34 students · Maths marks sheets</span>
          <span style={{ fontSize: 12, color: C.onNavyMuted }}>Only your classes are downloaded. No fee details.</span>
        </div>
      </div>
    </Phone>
  );
  return (
    <Phone bg={C.welcome}>
      <div style={{ padding: '22px 22px 0' }}><img src={A.logoLight} alt="Vidya Budget School" style={{ width: 150, height: 49, display: 'block' }} /></div>
      <div style={{ flexGrow: 1, padding: '28px 22px 24px', display: 'flex', flexDirection: 'column', gap: 20, ...rise('vRise14') }}>
        {step === 'code' && (
          <>
            <Eyebrow>Join my school</Eyebrow>
            <TwoLine l1="Namaste, Meena." l2="Scan your invitation." size={36} track="-0.02em" />
            <button type="button" data-hl="scan" style={{ height: 150, borderRadius: 14, border: `1.5px dashed ${C.lineStrong}`, background: C.white, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 10, color: C.ink }}>
              <Icon name="qr" size={40} sw={1.4} color={C.accent} /><span style={{ fontWeight: 600, fontSize: 15 }}>Scan QR code</span><span style={{ fontSize: 12, color: C.muted }}>Point your camera at the Principal’s screen</span>
            </button>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, color: C.muted, fontSize: 12 }}><span style={{ flexGrow: 1, height: 1, background: C.line }} />or type the code<span style={{ flexGrow: 1, height: 1, background: C.line }} /></div>
            <Field value="K7M2-QX9D" mono focused hl="code" />
            <div style={{ marginTop: 'auto' }}><Btn full h={52} icon="arrow" hl="cta">Continue</Btn></div>
          </>
        )}
        {step === 'confirm' && (
          <>
            <Eyebrow>Check before joining</Eyebrow>
            <TwoLine l1="Is this your school?" size={34} track="-0.02em" />
            <div data-hl="school" style={{ borderRadius: 14, background: C.navy, color: C.white, padding: 20, display: 'flex', flexDirection: 'column', gap: 12 }}>
              <span style={{ fontFamily: SERIF, fontSize: 26, lineHeight: 1.1 }}>Saraswati Public School</span>
              <span style={{ fontSize: 13, color: C.onNavyMuted }}>Mehdipatnam, Hyderabad</span>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4, borderTop: '1px solid rgba(255,255,255,0.14)', paddingTop: 12 }}><Label dark>Check code</Label><span style={{ fontFamily: "'Geist', monospace", fontSize: 20, letterSpacing: '0.1em', color: C.gold }}>3F9A 1C7E</span><span style={{ fontSize: 12, color: C.onNavyMuted }}>Matches the code on the Principal’s screen</span></div>
            </div>
            <div data-hl="role" style={{ borderRadius: 12, background: C.white, border: `1px solid ${C.line}`, padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 6 }}>
              <Label>You will join as</Label><span style={{ fontSize: 15, fontWeight: 600 }}>Teacher · Class teacher V-A</span><span style={{ fontSize: 13, color: C.muted }}>Attendance for V-A · Marks for V-A Maths</span>
            </div>
            <div style={{ marginTop: 'auto' }}><Btn full h={52} icon="arrow" hl="cta">Join Saraswati Public School</Btn></div>
          </>
        )}
        {step === 'pin' && (
          <>
            <Eyebrow>Almost done</Eyebrow>
            <TwoLine l1="Create your PIN." l2="To unlock this phone." size={36} track="-0.02em" />
            <div data-hl="pin" style={{ padding: '22px 0', display: 'flex', justifyContent: 'center' }}><PinDots filled={4} total={4} /></div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, marginTop: 'auto' }}>
              {['1', '2', '3', '4', '5', '6', '7', '8', '9', '', '0', '⌫'].map((k, i) => (
                <button key={i} type="button" style={{ height: 60, borderRadius: 12, border: k ? `1px solid ${C.line}` : 0, background: k ? C.white : 'transparent', fontFamily: SERIF, fontSize: 26, color: C.ink }}>{k}</button>
              ))}
            </div>
          </>
        )}
      </div>
    </Phone>
  );
}

const TE = {
  'Wednesday, 23 September': 'బుధవారం, 23 సెప్టెంబర్', 'Good morning, Meena.': 'శుభోదయం, మీనా.', 'Two things are due.': 'ఈ రోజు రెండు పనులు ఉన్నాయి.',
  'V-A attendance': 'V-A హాజరు', 'Not submitted · 34 students': 'ఇంకా సమర్పించలేదు · 34 విద్యార్థులు', Start: 'ప్రారంభించు',
  'V-A Maths marks': 'V-A గణితం మార్కులు', 'Half Yearly · 12 of 34 entered': 'అర్ధ వార్షికం · 34 లో 12 నమోదు',
  Attendance: 'హాజరు', Marks: 'మార్కులు', 'Report cards': 'రిపోర్ట్ కార్డులు', 'My classes': 'నా తరగతులు', Students: 'విద్యార్థులు',
  'My requests': 'నా అభ్యర్థనలు', Inbox: 'ఇన్‌బాక్స్', Sync: 'సింక్', Profile: 'ప్రొఫైల్', 'Up to date · synced 2 min ago': 'తాజాగా ఉంది · 2 నిమిషాల క్రితం',
};
export function TeacherHome({ requestBadge = 1, secondCard = 'returned', lang = 'en' }) {
  const tiles = ['Attendance', 'Marks', 'Report cards', 'My classes', 'Students', 'My requests', 'Inbox', 'Sync', 'Profile'];
  const t = (x) => (lang === 'te' ? TE[x] || x : x);
  const head = lang === 'te' ? { fontFamily: "'Noto Sans Telugu', sans-serif", fontWeight: 500 } : null;
  return (
    <Phone bg={GRAD.teacher} dark>
      <div style={{ flexGrow: 1, overflow: 'auto', padding: '18px 16px 24px', display: 'flex', flexDirection: 'column', gap: 20 }}>
        <PhoneLogoTop />
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, ...rise('vRise14') }}>
          <Eyebrow color={C.onNavyMuted} dotColor={C.gold} size={10} track="0.16em">{t('Wednesday, 23 September')}</Eyebrow>
          {head ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}><h1 data-hl="telugu" style={{ margin: 0, ...head, fontSize: 30, lineHeight: 1.25, color: C.white }}>{t('Good morning, Meena.')}</h1><span style={{ ...head, fontSize: 22, color: C.gold }}>{t('Two things are due.')}</span></div>
          ) : <TwoLine l1="Good morning, Meena." l2="Two things are due." size={34} color={C.white} italicColor={C.gold} track="-0.02em" />}
        </div>
        <section data-hl="due" style={{ border: '1px solid rgba(255,255,255,0.12)', borderRadius: 18, background: 'rgba(255,255,255,0.04)', overflow: 'hidden', ...rise('vRise14', 80) }}>
          <a href="#" style={{ display: 'flex', alignItems: 'center', gap: 14, padding: 16, textDecoration: 'none', color: C.white }}>
            <span style={{ width: 10, height: 10, borderRadius: 5, background: C.gold, flexShrink: 0, animation: 'vPulse 2.2s ease-in-out infinite' }} />
            <span style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: 3 }}><span style={{ fontSize: 15, fontWeight: 500 }}>{t('V-A attendance')}</span><span style={{ fontSize: 12, color: C.onNavyMuted }}>{t('Not submitted · 34 students')}</span></span>
            <span data-hl="start" style={{ height: 38, padding: '0 14px', borderRadius: 19, background: C.gold, color: C.navy, fontSize: 13, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 6 }}>{t('Start')}<Icon name="arrow" size={14} sw={2} /></span>
          </a>
          <a href="#" style={{ display: 'flex', alignItems: 'center', gap: 14, padding: 16, textDecoration: 'none', color: C.white, borderTop: '1px solid rgba(255,255,255,0.10)' }}>
            <span style={{ width: 10, height: 10, borderRadius: 5, background: C.tealSoft, flexShrink: 0 }} />
            <span style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', gap: 3 }}>
              <span style={{ fontSize: 15, fontWeight: 500 }}>{secondCard === 'marks' ? t('V-A Maths marks') : 'Reply to the Principal'}</span>
              <span style={{ fontSize: 12, color: C.onNavyMuted }}>{secondCard === 'marks' ? t('Half Yearly · 12 of 34 entered') : 'Rahul Kumar’s attendance correction was returned'}</span>
            </span>
            <Icon name="chevronRight" size={18} sw={1.75} color={C.onNavyMuted} />
          </a>
        </section>
        <div data-hl="tiles" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, minmax(0,1fr))', gap: 10 }}>
          {tiles.map((tt, i) => (
            <a key={tt} href="#" data-hl={`tile-${tt}`} style={{ position: 'relative', height: 112, borderRadius: 16, background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.08)', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 10, textDecoration: 'none', color: C.onNavyStrong, fontSize: 13, ...rise('vRise14', 140 + i * 40, 480) }}>
              {tt === 'My requests' && requestBadge ? <span style={{ position: 'absolute', top: 10, right: 10, minWidth: 20, height: 20, borderRadius: 10, background: C.gold, color: C.navy, fontSize: 11, fontWeight: 700, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>{requestBadge}</span> : null}
              <TileIcon name={tt} />{t(tt)}
            </a>
          ))}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8, fontSize: 12, color: C.onNavyMuted }}><span style={{ width: 6, height: 6, borderRadius: 3, background: C.online }} />{t('Up to date · synced 2 min ago')}</div>
      </div>
    </Phone>
  );
}

const NAMES = ['Aadhya Sharma', 'Aarav Gupta', 'Ananya Reddy', 'Arjun Yadav', 'Diya Patel', 'Ishaan Khan', 'Kabir Joshi', 'Meera Nair', 'Mohammed Faiz', 'Pooja Verma', 'Rahul Kumar', 'Riya Verma', 'Saanvi Rao', 'Vivaan Singh', 'Aditi Mishra', 'Ayaan Qureshi', 'Bhavya Jain', 'Dev Malhotra', 'Fatima Sheikh', 'Gaurav Chauhan', 'Harini Iyer', 'Ira Kapoor', 'Karan Mehta', 'Lakshmi Pillai', 'Manav Tiwari', 'Nandini Das', 'Om Prakash', 'Prisha Agarwal', 'Reyansh Bose', 'Sara Thomas', 'Tanvi Kulkarni', 'Uday Rathore', 'Vanya Saxena', 'Zoya Ansari'];
export function AttendanceScreen({ marks, submitted = false, toast = false }) {
  const st = marks || NAMES.map((_, i) => (i >= 30 ? '' : i === 8 || i === 10 ? 'A' : 'P'));
  const cP = st.filter((x) => x === 'P').length, cA = st.filter((x) => x === 'A').length, cU = st.filter((x) => x === '').length;
  const seg = (c, col) => ({ height: '100%', transition: 'width .35s cubic-bezier(.2,.8,.2,1)', background: col, width: `${c / st.length * 100}%` });
  const btn = { width: 44, height: 44, borderRadius: 8, fontSize: 14, fontWeight: 600 };
  const off = { ...btn, border: `1px solid ${C.line}`, background: C.white, color: C.muted };
  const on = { P: { ...btn, border: `1px solid ${C.accent}`, background: C.accent, color: C.white }, A: { ...btn, border: `1px solid ${C.danger}`, background: C.danger, color: C.white } };
  return (
    <Phone>
      <PhoneHeader title="Attendance." sub="Class V-A · Wed, 23 Sep" pill={<NetPill hl="pill">{submitted ? 'Offline · 1 waiting to send' : 'Offline · saved on this phone'}</NetPill>}>
        <Counts hl="counts" items={[[cP, 'Present'], [cA, 'Absent'], [cU, 'Not marked', true]]} />
        <div style={{ height: 4, borderRadius: 2, background: 'rgba(255,255,255,0.12)', display: 'flex', gap: 2, overflow: 'hidden' }}><div style={seg(cP, C.tealSoft)} /><div style={seg(cA, C.dangerSoft)} /></div>
      </PhoneHeader>
      <div style={{ flexGrow: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '14px 16px 10px' }}>
          <Label>34 students · tap P or A</Label>
          {!submitted && <button type="button" data-hl="markall" style={{ height: 36, padding: '0 14px', borderRadius: 18, border: `1px solid ${C.navy}`, background: C.navy, color: C.white, fontSize: 13, fontWeight: 500, display: 'flex', alignItems: 'center', gap: 6 }}><Icon name="check" size={14} sw={2.2} color={C.gold} />Mark all present</button>}
        </div>
        <ul style={{ listStyle: 'none', margin: 0, padding: '0 16px 16px', display: 'flex', flexDirection: 'column', gap: 6 }}>
          {NAMES.slice(0, 9).map((n, i) => (
            <li key={n} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '7px 7px 7px 14px', borderRadius: 10, border: `1px solid ${st[i] ? C.line : C.goldLine}`, background: st[i] ? C.surface : C.unmarked, transition: 'background-color .18s ease' }}>
              <span style={{ width: 26, fontFamily: SERIF, fontSize: 16, color: C.muted }}>{i + 1}</span>
              <span style={{ flexGrow: 1, fontSize: 15, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{n}</span>
              <div style={{ display: 'flex', gap: 6 }}>
                <button type="button" aria-label={`${n} present`} aria-pressed={st[i] === 'P'} disabled={submitted} style={st[i] === 'P' ? on.P : off}>P</button>
                <button type="button" data-hl={`a-${i}`} aria-label={`${n} absent`} aria-pressed={st[i] === 'A'} disabled={submitted} style={st[i] === 'A' ? on.A : off}>A</button>
              </div>
            </li>
          ))}
        </ul>
      </div>
      {!submitted ? (
        <div style={{ flexShrink: 0, background: C.surface, borderTop: `1px solid ${C.line}`, padding: '12px 16px 16px', display: 'flex', flexDirection: 'column', gap: 8 }}>
          {cU > 0 && <span style={{ fontSize: 12, color: C.goldText, textAlign: 'center' }}>{cU} students not marked yet</span>}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.6fr', gap: 10 }}>
            <button type="button" style={{ height: 50, borderRadius: 6, border: `1px solid ${C.lineStrong}`, background: 'transparent', color: C.ink, fontSize: 15, fontWeight: 500 }}>Save draft</button>
            <button type="button" data-hl="submit" disabled={cU > 0} style={{ height: 50, borderRadius: 6, border: 0, fontSize: 15, fontWeight: 500, background: cU > 0 ? C.lineStrong : C.accent, color: cU > 0 ? C.muted : C.white }}>Submit attendance</button>
          </div>
        </div>
      ) : (
        <div data-hl="submittedbar" style={{ flexShrink: 0, background: C.navy, color: C.white, padding: 16, display: 'flex', alignItems: 'center', gap: 12, ...rise('vIn10', 0, 300) }}>
          <span style={{ width: 40, height: 40, borderRadius: 20, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, animation: 'vPopAtt 400ms ease both' }}><Icon name="clock" size={20} sw={1.8} /></span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}><span style={{ fontFamily: SERIF, fontSize: 19 }}>Submitted, saved on this phone.</span><span style={{ fontSize: 12, color: C.onNavyMuted }}>It will reach the school when you are online. Changes now need a correction request.</span></div>
        </div>
      )}
      {toast && <Toast>Draft saved on this phone</Toast>}
    </Phone>
  );
}

const MARKS_VA = [['Aadhya Sharma', '78'], ['Aarav Gupta', '64'], ['Ananya Reddy', '91'], ['Arjun Yadav', 'AB'], ['Diya Patel', '85'], ['Ishaan Khan', '57'], ['Kabir Joshi', '']];
export function MarksScreen({ state = 'entering' }) {
  const submitted = state === 'submitted';
  const rows = MARKS_VA.map(([n, v], i) => [n, i === 6 && state !== 'entering' ? '72' : v]);
  const entered = state === 'entering' ? 33 : 34;
  return (
    <Phone>
      <PhoneHeader title="Marks." sub="Half Yearly · V-A Maths" pill={submitted ? <OnlinePill>Confirmed by school server</OnlinePill> : <OnlinePill>Online · synced just now</OnlinePill>}>
        <Counts hl="counts" items={[[entered, 'Entered'], [1, 'Absent (AB)'], [34 - entered, 'Missing', true]]} />
      </PhoneHeader>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '14px 16px 10px' }}><Label>Out of 100 · blank is not zero</Label></div>
      <ul style={{ listStyle: 'none', margin: 0, padding: '0 16px 16px', display: 'flex', flexDirection: 'column', gap: 6, flexGrow: 1, overflow: 'hidden' }}>
        {rows.map(([n, v], i) => {
          const focus = i === 6 && state === 'entering';
          return (
            <li key={n} data-hl={i === 6 ? 'focusrow' : undefined} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '7px 7px 7px 14px', borderRadius: 10, border: `1px solid ${v ? C.line : C.goldLine}`, background: v ? C.surface : C.unmarked }}>
              <span style={{ width: 26, fontFamily: SERIF, fontSize: 16, color: C.muted }}>{i + 1}</span>
              <span style={{ flexGrow: 1, fontSize: 15 }}>{n}</span>
              {v === 'AB' ? <span style={{ width: 96, height: 44, borderRadius: 8, background: C.partBg, color: C.partFg, display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 600 }}>AB</span>
                : <span style={{ width: 96, height: 44, borderRadius: 8, border: focus ? `1.5px solid ${C.accent}` : `1px solid ${C.line}`, boxShadow: focus ? `0 0 0 4px ${acc(0.12)}` : 'none', background: submitted ? C.panel : C.white, display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: SERIF, fontSize: 22, color: v ? C.ink : '#8A97A3', fontVariantNumeric: 'lining-nums tabular-nums' }}>{v || '—'}{submitted && <Icon name="lock" size={12} sw={2} color={C.muted} style={{ marginLeft: 6 }} />}</span>}
            </li>
          );
        })}
      </ul>
      {!submitted ? (
        <div style={{ flexShrink: 0, background: C.surface, borderTop: `1px solid ${C.line}`, padding: '12px 16px 16px', display: 'flex', flexDirection: 'column', gap: 8 }}>
          {state === 'entering' ? <span style={{ fontSize: 12, color: C.goldText, textAlign: 'center' }}>1 student has no marks yet</span> : <span style={{ fontSize: 12, color: C.accent, textAlign: 'center' }}>All 34 students entered</span>}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.6fr', gap: 10 }}>
            <button type="button" style={{ height: 50, borderRadius: 6, border: `1px solid ${C.lineStrong}`, background: 'transparent', color: C.ink, fontSize: 15, fontWeight: 500 }}>Save draft</button>
            <button type="button" data-hl="submit" disabled={state === 'entering'} style={{ height: 50, borderRadius: 6, border: 0, fontSize: 15, fontWeight: 500, background: state === 'entering' ? C.lineStrong : C.accent, color: state === 'entering' ? C.muted : C.white }}>Submit V-A Maths</button>
          </div>
        </div>
      ) : (
        <div data-hl="submittedbar" style={{ flexShrink: 0, background: C.navy, color: C.white, padding: 16, display: 'flex', alignItems: 'center', gap: 12, ...rise('vIn10', 0, 300) }}>
          <span style={{ width: 40, height: 40, borderRadius: 20, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, animation: 'vPopAtt 400ms ease both' }}><Icon name="lock" size={18} sw={1.8} /></span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}><span style={{ fontFamily: SERIF, fontSize: 19 }}>V-A Maths submitted.</span><span style={{ fontSize: 12, color: C.onNavyMuted }}>Locked. Other subjects are not affected. Changes need the Principal’s approval.</span></div>
        </div>
      )}
    </Phone>
  );
}

export function CorrectionScreen({ state = 'form' }) {
  return (
    <Phone>
      <PhoneHeader title="Correction." sub="Attendance · 19 Sep" pill={<OnlinePill>Online</OnlinePill>} />
      <div style={{ flexGrow: 1, padding: '18px 16px', display: 'flex', flexDirection: 'column', gap: 16 }}>
        <div data-hl="record" style={{ borderRadius: 12, background: C.surface, border: `1px solid ${C.line}`, padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 4 }}>
          <Label>Student</Label><span style={{ fontSize: 16, fontWeight: 600 }}>Rahul Kumar · Roll 11 · V-A</span><span style={{ fontSize: 13, color: C.muted }}>Sheet for Fri, 19 Sep is locked</span>
        </div>
        <div data-hl="change" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <div style={{ borderRadius: 12, border: `1px solid ${C.line}`, background: C.white, padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 4 }}><Label>Now</Label><span style={{ fontFamily: SERIF, fontSize: 24, color: C.danger }}>Absent</span></div>
          <div style={{ borderRadius: 12, border: `1.5px solid ${C.accent}`, background: acc(0.06), padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 4 }}><Label>Change to</Label><span style={{ fontFamily: SERIF, fontSize: 24, color: C.accent }}>Present</span></div>
        </div>
        <Field label="Reason for the Principal" textarea h={110} value="He came late after a doctor’s visit; I marked him absent by mistake. The note from his parents is with me." hl="reason" focused={state === 'form'} />
        <div style={{ marginTop: 'auto' }}>{state === 'form' ? <Btn full h={52} icon="arrow" hl="send">Send to Principal</Btn> : <Btn full h={52} variant="secondary" disabled>Sent · waiting for approval</Btn>}</div>
      </div>
      {state === 'sent' && <Toast bottom={100}>Sent to the Principal</Toast>}
    </Phone>
  );
}

export function RequestsScreen({ state = 'pending' }) {
  const approved = state === 'approved';
  const items = [
    ['Attendance correction', 'Rahul Kumar · 19 Sep · Absent → Present', approved ? ['Approved · applied', 'paid'] : ['Waiting for the Principal', 'partpaid'], approved ? 'Priya Sharma approved it just now' : 'Sent 10 minutes ago', true],
    ['Marks correction', 'Half Yearly · VII-A Maths · Om Prakash 45 → 54', ['Approved · applied', 'paid'], 'Last week', false],
  ];
  return (
    <Phone>
      <PhoneHeader title="My requests." sub={approved ? 'One decision today.' : 'Nothing is changed until approved.'} pill={<OnlinePill>Online</OnlinePill>} />
      {approved && (
        <div data-hl="banner" style={{ margin: '14px 16px 0', borderRadius: 12, background: C.navy, color: C.white, padding: '14px 16px', display: 'flex', gap: 12, alignItems: 'center', ...rise('vIn10', 0, 300) }}>
          <span style={{ width: 36, height: 36, borderRadius: 18, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}><Icon name="check" size={18} sw={2} /></span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}><span style={{ fontFamily: SERIF, fontSize: 18 }}>Approved by Priya Sharma.</span><span style={{ fontSize: 12, color: C.onNavyMuted }}>Rahul Kumar is now Present on 19 Sep.</span></div>
        </div>
      )}
      <div style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: 10 }}>
        {items.map(([t, s, [p, tone], when, first]) => (
          <div key={t} data-hl={first ? 'req' : undefined} style={{ borderRadius: 14, background: C.surface, border: `1px solid ${C.line}`, padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 10 }}>
            <Pill tone={tone} style={{ alignSelf: 'flex-start' }}>{p}</Pill>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}><span style={{ fontWeight: 500 }}>{t}</span><span style={{ fontSize: 13, color: C.muted }}>{s}</span></div>
            <span style={{ fontSize: 12, color: C.muted, display: 'flex', alignItems: 'center', gap: 6 }}><Icon name="clock" size={13} sw={1.8} />{when}</span>
          </div>
        ))}
      </div>
    </Phone>
  );
}

/* ================================================================== SCREENS: new modules */
function Toggle({ on, locked }) {
  return (
    <span style={{ width: 44, height: 26, borderRadius: 13, background: on ? (locked ? C.lineStrong : C.accent) : C.radioOff, position: 'relative', flexShrink: 0, transition: 'background-color .18s ease' }}>
      <span style={{ position: 'absolute', top: 3, left: on ? 21 : 3, width: 20, height: 20, borderRadius: 10, background: C.white, boxShadow: '0 1px 3px rgba(11,26,51,0.25)', transition: 'left .18s ease' }} />
    </span>
  );
}
function Table({ cols, head, rows, hlRow, hlName = 'row', dense }) {
  return (
    <div style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, overflow: 'hidden' }}>
      <div style={{ display: 'grid', gridTemplateColumns: cols, padding: '12px 24px', borderBottom: `1px solid ${C.track}` }}>{head.map((h, i) => <Label key={i}>{h}</Label>)}</div>
      {rows.map((r, i) => (
        <div key={i} data-hl={i === hlRow ? hlName : undefined} style={{ display: 'grid', gridTemplateColumns: cols, alignItems: 'center', padding: dense ? '11px 24px' : '14px 24px', borderBottom: `1px solid ${C.track}`, background: i === hlRow ? acc(0.05) : 'transparent', fontVariantNumeric: 'tabular-nums' }}>
          {r.map((c, j) => <span key={j} style={{ minWidth: 0 }}>{c}</span>)}
        </div>
      ))}
    </div>
  );
}
const Sub = ({ children }) => <span style={{ display: 'block', fontSize: 12, color: C.muted, marginTop: 2 }}>{children}</span>;
const money = (v) => (v < 0 ? '−₹' : '₹') + Math.abs(v).toLocaleString('en-IN');
function Strip({ items, hl }) {
  return (
    <div data-hl={hl} style={{ display: 'grid', gridTemplateColumns: `repeat(${items.length}, minmax(0,1fr))`, background: C.panel, borderRadius: 16 }}>
      {items.map(([v, l, n], i) => (
        <div key={l} style={{ display: 'flex', flexDirection: 'column', gap: 8, padding: '20px 24px', borderLeft: i ? `1px solid ${C.lineStat}` : 0 }}>
          <span style={{ fontFamily: SERIF, fontSize: 36, lineHeight: 1, letterSpacing: '-0.02em', fontVariantNumeric: 'lining-nums tabular-nums' }}>{v}</span>
          <span style={{ fontSize: 14 }}>{l}</span>{n && <span style={{ fontSize: 12, color: C.muted }}>{n}</span>}
        </div>
      ))}
    </div>
  );
}
const Main = ({ children, gap = 24 }) => <main style={{ flexGrow: 1, padding: '32px 40px', display: 'flex', flexDirection: 'column', gap, minHeight: 0 }}>{children}</main>;

/* ---- Settings: payments (UPI), Google Drive, languages & modules */
export function SettingsScreen({ tab = 'payments' }) {
  const nav = [['School', 'globe'], ['Session & classes', 'attendance'], ['Payments (UPI)', 'fees'], ['Google Drive', 'drive'], ['Languages & modules', 'settings'], ['Printing', 'print'], ['Security', 'shield']];
  const cur = { payments: 'Payments (UPI)', drive: 'Google Drive', modules: 'Languages & modules' }[tab];
  const sub = { payments: 'How parents pay you.', drive: 'Where your data is kept.', modules: 'Turn on what you need.' }[tab];
  return (
    <Desktop role="principal" active="settings">
      <Main>
        <PageTitle eyebrow="Settings · Principal only" l1="Settings." l2={sub} />
        <div style={{ display: 'grid', gridTemplateColumns: '280px 1fr', gap: 24, flexGrow: 1, minHeight: 0 }}>
          <Card>
            <div style={{ padding: 8, display: 'flex', flexDirection: 'column', gap: 2 }}>
              {nav.map(([n, ic]) => (
                <span key={n} style={{ display: 'flex', alignItems: 'center', gap: 11, height: 42, padding: '0 14px', borderRadius: 8, background: n === cur ? acc(0.08) : 'transparent', color: n === cur ? C.accent : C.ink, fontWeight: n === cur ? 600 : 500 }}><Icon name={ic} size={17} />{n}</span>
              ))}
            </div>
          </Card>
          {tab === 'payments' && (
            <Card eyebrow="Payments" title="School UPI">
              <div style={{ padding: '4px 24px 24px', display: 'grid', gridTemplateColumns: '1fr 300px', gap: 28 }}>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 18 }}>
                  <Field label="School UPI ID" value="saraswatischool@okhdfcbank" focused hl="upi" hint="From the school's bank or UPI app. Money goes straight to the school; Vidya never handles it." />
                  <Field label="Name parents will see" value="Saraswati Public School" />
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                    <span style={{ fontSize: 13, fontWeight: 500 }}>Show the payment QR on</span>
                    {[['Fee receipts (for the balance)', true], ['Fee reminders by email and WhatsApp', true], ['Printed dues lists', false]].map(([t, on]) => (
                      <span key={t} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 14px', borderRadius: 8, border: `1px solid ${C.line}`, background: C.white }}>{t}<Toggle on={on} /></span>
                    ))}
                  </div>
                </div>
                <div data-hl="qrpreview" style={{ borderRadius: 14, background: C.navy, color: C.white, padding: 22, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14, textAlign: 'center', ...rise('vIn8', 0, 300) }}>
                  <Label dark>Preview for one student</Label>
                  <div style={{ background: C.white, padding: 12, borderRadius: 10 }}><FakeQR size={160} seed={11} /></div>
                  <span style={{ fontFamily: SERIF, fontSize: 24 }}>Scan to pay <span style={{ fontStyle: 'italic', color: C.gold }}>₹2,100</span></span>
                  <span style={{ fontSize: 12, color: C.onNavyMuted }}>Kavya Singh · VI-B · Term 2 balance. The amount and name are filled in for every student.</span>
                </div>
              </div>
            </Card>
          )}
          {tab === 'drive' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
              {[['sync', 'School sync account', 'vidya.saraswatischool@gmail.com', 'Shared by the school. Passes encrypted changes between phones and this PC when staff are away from school Wi-Fi.', [['9', 'devices connected'], ['12 s', 'since last check'], ['Encrypted', 'nobody can read the files']]],
                ['backup', 'Backup account (private)', 'priya.sharma@gmail.com', 'Only the Principal knows this account. Keeps encrypted daily backups that only the recovery key can open.', [['42', 'backups kept'], ['6:02 AM', 'last verified'], ['1.8 GB', 'of 15 GB free space']]]].map(([hl, t, email, desc, stats]) => (
                <section key={hl} data-hl={hl} style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, padding: '22px 24px', display: 'flex', flexDirection: 'column', gap: 16, ...rise('vIn8', hl === 'sync' ? 0 : 120, 300) }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
                    <span style={{ width: 44, height: 44, borderRadius: 22, background: C.navy, color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Icon name={hl === 'sync' ? 'sync' : 'backups'} size={20} /></span>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 2, flexGrow: 1 }}><span style={{ fontFamily: SERIF, fontSize: 24 }}>{t}</span><span style={{ fontSize: 14, color: C.muted }}>{email}</span></div>
                    <Pill tone="paid">Connected</Pill>
                  </div>
                  <span style={{ fontSize: 14, lineHeight: 1.5, color: C.ink }}>{desc}</span>
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', background: C.panel, borderRadius: 10 }}>
                    {stats.map(([v, l], i) => <div key={l} style={{ padding: '12px 16px', borderLeft: i ? `1px solid ${C.lineStat}` : 0 }}><span style={{ fontFamily: SERIF, fontSize: 24 }}>{v}</span><Sub>{l}</Sub></div>)}
                  </div>
                </section>
              ))}
            </div>
          )}
          {tab === 'modules' && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
              <Card eyebrow="Languages" title="English, Hindi and Telugu" hl="languages">
                <div style={{ padding: '0 24px 22px', display: 'flex', gap: 10 }}>
                  {[['English', false], ['हिंदी', false], ['తెలుగు', true]].map(([l, on]) => <span key={l} style={{ height: 44, padding: '0 22px', borderRadius: 22, border: `1.5px solid ${on ? C.accent : C.line}`, background: on ? acc(0.08) : C.white, display: 'flex', alignItems: 'center', fontSize: 17, fontWeight: 500, color: on ? C.accent : C.ink }}>{l}</span>)}
                  <span style={{ marginLeft: 12, alignSelf: 'center', fontSize: 13, color: C.muted }}>Each staff member picks their own. Receipts and report cards follow the school's choice.</span>
                </div>
              </Card>
              <Card eyebrow="Modules" title="Switch on what your school uses" hl="modules">
                {[['Fees, receipts & dues', 'Core', true, true], ['School accounts', 'Expenses, cash book, profit, salaries', true], ['Classroom', 'Timetable, homework & notes, exams, calendar', true], ['Staff HR', 'Staff attendance and leave', true], ['Circulars & notices', 'To parents and staff', true],
                  ['Automatic WhatsApp', 'Optional · the school pays Meta about ₹0.12 per message', false], ['School store', 'Optional · books and uniforms with stock', false]].map(([t, d, on, locked]) => (
                  <div key={t} data-hl={t === 'Automatic WhatsApp' ? 'optional' : undefined} style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '13px 24px', borderTop: `1px solid ${C.track}` }}>
                    <div style={{ flexGrow: 1 }}><span style={{ fontWeight: 500 }}>{t}</span><Sub>{d}</Sub></div>
                    {!on && <Pill tone="partpaid">Optional</Pill>}<Toggle on={on} locked={locked} />
                  </div>
                ))}
              </Card>
            </div>
          )}
        </div>
      </Main>
    </Desktop>
  );
}

/* ---- Fees admin: structure with instalments, dues, reminders */
export function FeesAdminScreen({ state = 'structure' }) {
  if (state === 'structure') return (
    <Desktop role="principal" active="fees">
      <Main>
        <PageTitle eyebrow="Fees · 2026–27 · Class VI" l1="Fee structure." l2="Split into instalments." actions={<Btn icon="plus">Add fee head</Btn>} />
        <Table cols="1.4fr 1fr 2.6fr 1fr" head={['Fee head', 'Amount', 'Instalments and due dates', 'Applies to']} hlRow={0} hlName="instalments" rows={[
          [<b key="a" style={{ fontWeight: 600 }}>Tuition</b>, '₹12,000 / year', <span key="b" style={{ display: 'flex', gap: 8 }}>{[['15 Apr', '₹4,000'], ['15 Aug', '₹4,000'], ['15 Dec', '₹4,000']].map(([d, a]) => <span key={d} style={{ display: 'flex', flexDirection: 'column', padding: '6px 12px', borderRadius: 8, background: acc(0.08), color: C.accent, fontSize: 13 }}><b style={{ fontWeight: 600 }}>{a}</b>{d}</span>)}</span>, 'All students'],
          [<b key="a" style={{ fontWeight: 600 }}>Exam fee</b>, '₹1,000 / year', <span key="b" style={{ display: 'flex', gap: 8 }}>{[['15 Sep', '₹500'], ['15 Feb', '₹500']].map(([d, a]) => <span key={d} style={{ display: 'flex', flexDirection: 'column', padding: '6px 12px', borderRadius: 8, background: C.panel, fontSize: 13 }}><b style={{ fontWeight: 600 }}>{a}</b>{d}</span>)}</span>, 'All students'],
          [<b key="a" style={{ fontWeight: 600 }}>Transport</b>, '₹200 / month', <span key="b" style={{ color: C.muted }}>Every month, due on the 10th</span>, 'Bus students'],
          [<b key="a" style={{ fontWeight: 600 }}>Annual charges</b>, '₹1,500 once', <span key="b" style={{ color: C.muted }}>Due at admission or 15 April</span>, 'All students'],
        ]} />
        <div style={{ display: 'flex', gap: 16 }}>
          <div style={{ flex: 1, borderRadius: 12, background: C.panel, padding: '16px 20px' }}><Label>Changing amounts</Label><p style={{ margin: '6px 0 0', fontSize: 14, lineHeight: 1.5 }}>Vidya shows which unpaid instalments will change before you save. Paid instalments never change.</p></div>
          <div style={{ flex: 1, borderRadius: 12, background: C.panel, padding: '16px 20px' }}><Label>Reminders</Label><p style={{ margin: '6px 0 0', fontSize: 14, lineHeight: 1.5 }}>Parents can be reminded 3 days before each due date, by email or WhatsApp.</p></div>
        </div>
      </Main>
    </Desktop>
  );
  const dues = [['Kavya Singh', 'VI-B', 'Tuition · 2 of 3', '15 Aug', 2100, 'overdue'], ['Aarav Gupta', 'V-A', 'Tuition · 2 of 3', '15 Aug', 4000, 'overdue'], ['Riya Verma', 'V-A', 'Exam fee · 1 of 2', '15 Sep', 500, 'overdue'], ['Om Prakash', 'VII-A', 'Transport · Sep', '10 Sep', 200, 'overdue'], ['Diya Patel', 'V-A', 'Tuition · 2 of 3', '15 Aug', 1500, 'overdue'], ['Karan Mehta', 'VIII-A', 'Exam fee · 1 of 2', '15 Sep', 500, 'overdue']];
  return (
    <Desktop role="principal" active="fees">
      <Main>
        <PageTitle eyebrow="Fees · as of today" l1="Dues." l2="Who owes what." actions={<><Btn variant="secondary" icon="chat">WhatsApp one by one</Btn><Btn icon="mail" hl="emailall">Email all 212 parents</Btn></>} />
        <Strip items={[['₹6,84,200', 'total due this term'], ['212', 'students with dues'], ['64', 'overdue instalments'], ['₹48,500', 'collected today']]} />
        <Table cols="1.6fr 0.6fr 1.3fr 0.8fr 0.8fr 1.3fr" head={['Student', 'Class', 'Instalment', 'Due date', 'Amount', 'Remind']} hlRow={0} hlName="duerow" rows={dues.map(([n, c, i, d, a], k) => [
          <span key="n"><b style={{ fontWeight: 500 }}>{n}</b><Sub>Guardian · {['Rajesh Singh', 'Neha Gupta', 'Suresh Verma', 'Ravi Prakash', 'Kiran Patel', 'Arun Mehta'][k]}</Sub></span>, c, i,
          <Pill key="d" tone="unpaid">{d}</Pill>, <b key="a" style={{ fontWeight: 600 }}>{money(a)}</b>,
          <span key="r" style={{ display: 'flex', gap: 8 }}><Btn variant="secondary" h={34} icon="mail" style={{ fontSize: 13 }} hl={k === 0 ? 'remind' : undefined}>Email</Btn><Btn variant="secondary" h={34} icon="chat" style={{ fontSize: 13 }}>WhatsApp</Btn></span>,
        ])} />
      </Main>
      {state === 'reminder' && <Overlay />}
      {state === 'reminder' && (
        <SheetPanel eyebrow="Fee reminder" title="Kavya Singh · VI-B" sub="Guardian Rajesh Singh · rajesh.singh@gmail.com">
          <Segmented options={['Email (free)', 'WhatsApp (free)', 'Automatic']} value="Email (free)" />
          <div style={{ display: 'flex', gap: 8 }}>{[['English', true], ['हिंदी', false], ['తెలుగు', false]].map(([l, on]) => <Chip key={l} on={on}>{l}</Chip>)}</div>
          <div data-hl="preview" style={{ borderRadius: 12, border: `1px solid ${C.line}`, background: C.white, padding: 18, display: 'flex', flexDirection: 'column', gap: 10, fontSize: 14, lineHeight: 1.55, ...rise('vIn8', 0, 300) }}>
            <span style={{ fontSize: 12, color: C.muted }}>From: Saraswati Public School (school’s own Gmail)</span>
            <span style={{ fontWeight: 600 }}>Fee reminder – Kavya Singh (VI-B)</span>
            <span>Dear Rajesh Singh, Term 2 tuition of <b>₹2,100</b> for Kavya was due on 15 August. Please pay at the school office or scan the QR below.</span>
            <div data-hl="qr" style={{ display: 'flex', alignItems: 'center', gap: 14, padding: 10, borderRadius: 8, background: C.bg }}><FakeQR size={88} seed={11} /><span style={{ fontSize: 13 }}><b>Scan to pay ₹2,100</b><Sub>UPI · saraswatischool@okhdfcbank</Sub></span></div>
            <span style={{ fontSize: 12, color: C.muted }}>— Saraswati Public School, Mehdipatnam</span>
          </div>
          <div style={{ marginTop: 'auto', display: 'flex', flexDirection: 'column', gap: 8 }}>
            <Btn full h={52} icon="mail" hl="send">Send email</Btn>
            <span style={{ fontSize: 12, color: C.muted, textAlign: 'center' }}>Sent from the school’s Gmail · free · up to about 500 a day</span>
          </div>
        </SheetPanel>
      )}
    </Desktop>
  );
}

/* ---- Accounts: cash book, expense, profit summary */
const CASH = [['09:10', 'R-A2-0401', 'Fee · Aarav Gupta', 3000, 0], ['10:05', 'V-0087', 'Electricity bill · September', 0, 6850], ['11:42', 'R-A2-0419', 'Fee · Kavya Singh', 1000, 0], ['12:30', 'V-0088', 'Repairs · classroom fan, V-B', 0, 1200], ['13:15', 'V-0089', 'Stationery · registers and chalk', 0, 4300], ['All day', '21 receipts', 'Fees · other students', 44500, 0]];
export function AccountsScreen({ state = 'cashbook' }) {
  if (state === 'profit') {
    const months = [['Apr', 980000, 620000], ['May', 210000, 540000], ['Jun', 840000, 610000], ['Jul', 520000, 600000], ['Aug', 760000, 620000], ['Sep', 410000, 430000]];
    return (
      <Desktop role="principal" active="accounts">
        <Main gap={22}>
          <PageTitle eyebrow="Accounts · 2026–27 so far" l1="Profit summary." l2="Month by month." actions={<Btn variant="secondary" icon="print">Print</Btn>} />
          <Strip hl="profitstrip" items={[['₹37,20,000', 'income', 'fees and other income'], ['₹34,20,000', 'expenses', 'salaries, rent, bills, repairs'], ['₹3,00,000', 'surplus', 'income minus expenses'], ['₹6,84,200', 'fees still due', 'this term']]} />
          <section data-hl="chart" style={{ background: C.navy, color: C.white, borderRadius: 16, padding: '20px 28px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}><h2 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 24 }}>Income and expenses</h2><span style={{ display: 'flex', gap: 18, fontSize: 12, color: C.onNavyMuted }}><span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><span style={{ width: 10, height: 10, borderRadius: 2, background: C.tealSoft }} />Income</span><span style={{ display: 'flex', alignItems: 'center', gap: 6 }}><span style={{ width: 10, height: 10, borderRadius: 2, background: C.gold }} />Expenses</span></span></div>
            <div style={{ display: 'flex', alignItems: 'flex-end', gap: 30, height: 200, paddingTop: 10 }}>
              {months.map(([m, inc, exp], i) => (
                <div key={m} style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
                  <div style={{ display: 'flex', alignItems: 'flex-end', gap: 6, height: 170 }}>
                    <div style={{ width: 26, height: inc / 1000000 * 170, background: C.tealSoft, borderRadius: '4px 4px 2px 2px', transformOrigin: 'bottom', animation: `vGrow 700ms ${200 + i * 80}ms ${EASE} both` }} />
                    <div style={{ width: 26, height: exp / 1000000 * 170, background: C.gold, borderRadius: '4px 4px 2px 2px', transformOrigin: 'bottom', animation: `vGrow 700ms ${240 + i * 80}ms ${EASE} both` }} />
                  </div>
                  <span style={{ fontSize: 12, color: C.onNavyMuted }}>{m}</span>
                </div>
              ))}
            </div>
          </section>
          <Table dense cols="1fr 1fr 1fr 1fr" head={['Month', 'Income', 'Expenses', 'Surplus']} rows={months.map(([m, i, e]) => [m, money(i), money(e), <b key="s" style={{ fontWeight: 600, color: i - e < 0 ? C.danger : C.accent }}>{money(i - e)}</b>])} />
        </Main>
      </Desktop>
    );
  }
  let bal = 38400;
  const rows = CASH.map(([tm, v, d, i, o]) => { bal += i - o; return [tm, <span key="v" style={{ color: v.startsWith('V') ? C.goldText : C.accent, fontWeight: 500 }}>{v}</span>, d, i ? money(i) : '', o ? money(o) : '', <b key="b" style={{ fontWeight: 600 }}>{money(bal)}</b>]; });
  return (
    <Desktop role="principal" active="accounts">
      <Main>
        <PageTitle eyebrow="Accounts · Wednesday, 23 September" l1="Cash book." l2="Every rupee in and out." actions={<><Btn variant="secondary" icon="print">Print day</Btn><Btn icon="plus" hl="addexp">Record expense</Btn></>} />
        <Strip hl="cashstrip" items={[['₹38,400', 'opening balance'], ['₹48,500', 'money in today', '23 fee receipts'], ['₹12,350', 'money out today', '3 expense vouchers'], ['₹74,550', 'cash and bank in hand', 'opening + in − out']]} />
        <Table cols="0.7fr 1fr 2.2fr 1fr 1fr 1fr" head={['Time', 'Receipt / voucher', 'Details', 'Money in', 'Money out', 'Balance']} hlRow={3} hlName="exprow" rows={rows} />
      </Main>
      {state === 'expense' && <Overlay />}
      {state === 'expense' && (
        <SheetPanel eyebrow="Record an expense" title="Repairs" sub="Voucher V-0088 is given when you save">
          <div data-hl="category" style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><span style={{ fontSize: 13, fontWeight: 500 }}>Category</span><div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>{['Electricity', 'Rent', 'Repairs', 'Stationery', 'Salary', 'Other'].map((c) => <Chip key={c} on={c === 'Repairs'}>{c}</Chip>)}</div></div>
          <div data-hl="amount" style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><span style={{ fontSize: 13, fontWeight: 500 }}>Amount paid</span>
            <div style={{ display: 'flex', alignItems: 'center', height: 62, borderRadius: 6, background: C.white, border: `1.5px solid ${C.accent}`, boxShadow: `0 0 0 4px ${acc(0.12)}` }}><span style={{ fontFamily: SERIF, fontSize: 28, color: C.muted, paddingLeft: 16 }}>₹</span><span style={{ fontFamily: SERIF, fontSize: 30, paddingLeft: 6 }}>1200</span></div></div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><span style={{ fontSize: 13, fontWeight: 500 }}>Paid by</span><Segmented options={['Cash', 'UPI', 'Bank']} value="Cash" /></div>
          <Field label="Details" value="Classroom fan, V-B · Ravi Electricals" h={46} />
          <div data-hl="bill" style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 14px', borderRadius: 8, border: `1px dashed ${C.lineStrong}`, background: C.white }}><Icon name="image" size={20} color={C.accent} /><span style={{ flexGrow: 1 }}>bill-fan-repair.jpg<Sub>Photo of the bill · 180 KB</Sub></span><Icon name="check" size={18} sw={2} color={C.accent} /></div>
          <div style={{ marginTop: 'auto' }}><Btn full h={52} icon="arrow" hl="save">Save expense</Btn></div>
        </SheetPanel>
      )}
    </Desktop>
  );
}

export function SalaryScreen() {
  const rows = [['Meena Iyer', 'Teacher', 18000, '25 / 26', 2000, 0, 'Paid'], ['Anita Rao', 'Teacher', 20000, '26 / 26', 0, 0, 'Paid'], ['R. Nair', 'Teacher', 17000, '24 / 26', 0, 1308, 'Pending'], ['Suresh Patel', 'Accountant', 16000, '26 / 26', 0, 0, 'Pending'], ['Lakshmi', 'Support staff', 8000, '26 / 26', 500, 0, 'Pending']];
  return (
    <Desktop role="principal" active="accounts">
      <Main>
        <PageTitle eyebrow="Accounts · September 2026" l1="Salary register." l2="Salary, advances, net pay." actions={<><Btn variant="secondary" icon="print">Print salary slips</Btn><Btn icon="check" hl="pay">Pay 3 pending</Btn></>} />
        <Strip items={[['₹79,000', 'total salaries'], ['₹2,500', 'advances recovered'], ['₹1,308', 'unpaid leave deducted'], ['₹75,192', 'net to pay']]} />
        <Table cols="1.6fr 1fr 1fr 1fr 0.9fr 1fr 1fr 0.9fr" head={['Staff', 'Role', 'Monthly', 'Days present', 'Advance', 'Deductions', 'Net pay', 'Status']} hlRow={2} hlName="deduction" rows={rows.map(([n, r, m, d, a, dd, st]) => [
          <b key="n" style={{ fontWeight: 500 }}>{n}</b>, r, money(m), d, a ? money(a) : '—', dd ? <span key="d">{money(dd)}<Sub>2 days unpaid leave</Sub></span> : '—', <b key="np" style={{ fontWeight: 600 }}>{money(m - a - dd)}</b>, <Pill key="s" tone={st === 'Paid' ? 'paid' : 'partpaid'}>{st}</Pill>])} />
        <span style={{ fontSize: 13, color: C.muted }}>Days present come from staff attendance and approved leave. Every salary paid also appears in the cash book as an expense.</span>
      </Main>
    </Desktop>
  );
}

/* ---- Timetable + substitutes */
const TT = [['Maths', 'MI'], ['English', 'RN'], ['Science', 'AR'], ['Hindi', 'SK'], ['Telugu', 'VL'], ['Social', 'PK']];
export function TimetableScreen({ state = 'week' }) {
  const days = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  return (
    <Desktop role="principal" active="timetable">
      <Main gap={20}>
        <PageTitle eyebrow="Academics · Class V-A" l1="Timetable." l2="Every period, every teacher." actions={<><Btn variant="secondary">V-A</Btn><Btn icon="staff" hl="subbtn">Substitutes today</Btn></>} />
        <div data-hl="grid" style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, overflow: 'hidden', display: 'grid', gridTemplateColumns: '110px repeat(6, 1fr)' }}>
          <span style={{ padding: '12px 16px' }} />
          {days.map((d) => <span key={d} style={{ padding: '12px 16px', borderLeft: `1px solid ${C.track}`, background: d === 'Wed' ? acc(0.08) : 'transparent', color: d === 'Wed' ? C.accent : C.ink, fontWeight: 600 }}>{d}{d === 'Wed' && <Sub>Today</Sub>}</span>)}
          {[1, 2, 3, 4, 5, 6].map((p) => (
            <React.Fragment key={p}>
              <span style={{ padding: '14px 16px', borderTop: `1px solid ${C.track}` }}><b style={{ fontFamily: SERIF, fontWeight: 400, fontSize: 20 }}>{p}</b><Sub>{['8:40', '9:25', '10:10', '11:10', '11:55', '12:40'][p - 1]}</Sub></span>
              {days.map((d, di) => {
                const [sub, t] = TT[(p + di * 2) % 6];
                const swap = d === 'Wed' && t === 'RN';
                return (
                  <span key={d} style={{ padding: '10px 12px', borderTop: `1px solid ${C.track}`, borderLeft: `1px solid ${C.track}`, background: d === 'Wed' ? acc(0.04) : 'transparent' }}>
                    <span data-hl={swap ? 'swapcell' : undefined} style={{ display: 'flex', flexDirection: 'column', gap: 2, padding: '8px 10px', borderRadius: 8, background: swap ? (state === 'assigned' ? acc(0.14) : C.partBg) : C.bg }}>
                      <b style={{ fontWeight: 600, fontSize: 13 }}>{sub}</b>
                      <span style={{ fontSize: 12, color: swap ? (state === 'assigned' ? C.accent : C.partFg) : C.muted }}>{swap ? (state === 'assigned' ? 'MI · substitute' : 'RN · on leave') : t}</span>
                    </span>
                  </span>
                );
              })}
            </React.Fragment>
          ))}
        </div>
      </Main>
      {state === 'substitute' && <Overlay />}
      {state === 'substitute' && (
        <SheetPanel eyebrow="Substitute · Wed 23 Sep" title="R. Nair is on leave" sub="Approved leave · 2 periods and VII-B attendance need cover">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {[['Period 2 · V-A English', '9:25'], ['Period 4 · VII-B English', '11:10'], ['VII-B attendance (class teacher)', 'Morning']].map(([t, tm]) => <div key={t} style={{ display: 'flex', justifyContent: 'space-between', padding: '11px 14px', borderRadius: 8, background: C.panel }}><span style={{ fontWeight: 500 }}>{t}</span><span style={{ color: C.muted }}>{tm}</span></div>)}
          </div>
          <div data-hl="free" style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><Label>Free in those periods</Label>
            <RadioCard on title="Meena Iyer" sub="Free in periods 2 and 4 · 3 periods today" /><RadioCard title="Anita Rao" sub="Free in period 4 only" />
          </div>
          <div style={{ borderRadius: 10, background: acc(0.08), padding: '12px 14px', fontSize: 13, color: C.accent }}>Meena can take VII-B attendance today only. Access ends automatically tonight.</div>
          <div style={{ marginTop: 'auto' }}><Btn full h={52} icon="check" hl="assign">Assign Meena Iyer</Btn></div>
        </SheetPanel>
      )}
    </Desktop>
  );
}

/* ---- Exams: seating + hall tickets */
export function ExamsScreen({ state = 'seating' }) {
  if (state === 'hallticket') return (
    <PrintFrame title="Print hall tickets" sub="A4 · 4 per page · Half Yearly · 670 students">
      <div data-hl="paper" style={{ width: 760, background: C.white, boxShadow: '0 30px 60px rgba(11,26,51,0.18)', padding: 24, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16, ...rise('vIn8', 0, 380) }}>
        {[['Kavya Singh', 'VI-B · Roll 14', 'Room 3 · Seat 14'], ['Aarav Gupta', 'V-A · Roll 2', 'Room 1 · Seat 3'], ['Ananya Reddy', 'V-A · Roll 3', 'Room 1 · Seat 5'], ['Om Prakash', 'VII-A · Roll 27', 'Room 4 · Seat 22']].map(([n, c, r], i) => (
          <div key={n} data-hl={i === 0 ? 'ticket' : undefined} style={{ border: `1px solid ${C.line}`, borderRadius: 8, padding: 14, display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}><img src={A.logoLight} alt="" style={{ width: 96, height: 31 }} /><span style={{ fontFamily: SERIF, fontStyle: 'italic', color: C.accent }}>Hall ticket</span></div>
            <span style={{ fontSize: 11, color: C.muted }}>Saraswati Public School · Half Yearly Exam 2026–27</span>
            <span style={{ fontFamily: SERIF, fontSize: 22 }}>{n}</span><span style={{ fontSize: 12 }}>{c}</span>
            <span style={{ alignSelf: 'flex-start', padding: '4px 10px', borderRadius: 6, background: C.navy, color: C.white, fontSize: 12, fontWeight: 600 }}>{r}</span>
            {[['Mon 21 Sep', 'English'], ['Tue 22 Sep', 'Maths'], ['Wed 23 Sep', 'Science']].map(([d, s]) => <div key={d} style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, borderTop: `1px solid ${C.track}`, paddingTop: 4 }}><span>{d}</span><span>{s}</span></div>)}
            <span style={{ fontSize: 10, color: C.muted, marginTop: 6 }}>Principal’s signature ____________</span>
          </div>
        ))}
      </div>
    </PrintFrame>
  );
  const seat = (r, c) => { const k = r * 5 + c; return k % 2 === 0 ? ['V-A', k / 2 + 1] : ['VI-B', (k + 1) / 2]; };
  return (
    <Desktop role="principal" active="marks">
      <Main gap={20}>
        <PageTitle eyebrow="Exams · Half Yearly 2026–27" l1="Seating plan." l2="Room by room." actions={<><Btn variant="secondary" icon="print">Seating charts</Btn><Btn icon="print" hl="halltickets">Print hall tickets</Btn></>} />
        <Strip items={[['670', 'students'], ['17', 'rooms'], ['2', 'classes per room', 'so neighbours never share a paper'], ['40', 'seats per room']]} />
        <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr', gap: 24 }}>
          <Card eyebrow="Room 1 · ground floor" title="V-A and VI-B, alternating" hl="room">
            <div style={{ padding: '4px 24px 22px', display: 'flex', flexDirection: 'column', gap: 10 }}>
              <span style={{ alignSelf: 'center', padding: '4px 30px', borderRadius: 4, background: C.navy, color: C.white, fontSize: 11, letterSpacing: '0.14em' }}>BOARD</span>
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: 8 }}>
                {Array.from({ length: 40 }).map((_, k) => { const [cl, r] = seat(Math.floor(k / 5), k % 5); return <span key={k} style={{ height: 34, borderRadius: 6, background: cl === 'V-A' ? acc(0.12) : C.partBg, color: cl === 'V-A' ? C.accent : C.partFg, fontSize: 11, fontWeight: 600, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>{cl} · {Math.ceil(r)}</span>; })}
              </div>
            </div>
          </Card>
          <Card eyebrow="All rooms" title="Rooms and invigilators">
            {[['Room 1', 'V-A + VI-B', 'Meena Iyer'], ['Room 2', 'V-B + VI-A', 'Anita Rao'], ['Room 3', 'VI-B + VII-A', 'R. Nair'], ['Room 4', 'VII-A + VIII-A', 'S. Khan'], ['Room 5', 'VIII-B + IX-A', 'V. Lakshmi']].map(([r, c, t]) => (
              <div key={r} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 24px', borderTop: `1px solid ${C.track}` }}><b style={{ fontWeight: 600, width: 64 }}>{r}</b><span style={{ flexGrow: 1 }}>{c}</span><span style={{ fontSize: 12, color: C.muted }}>{t}</span></div>
            ))}
          </Card>
        </div>
      </Main>
    </Desktop>
  );
}
function PrintFrame({ title, sub, children }) {
  return (
    <div className="vd" style={{ width: 1440, height: 1080, background: C.outer, fontFamily: SANS, color: C.ink, display: 'flex', flexDirection: 'column' }}>
      <div style={{ height: 64, background: C.navy, color: C.white, display: 'flex', alignItems: 'center', padding: '0 32px', gap: 16, flexShrink: 0 }}>
        <Icon name="print" size={20} color={C.gold} /><span style={{ fontFamily: SERIF, fontSize: 22 }}>{title}</span><span style={{ color: C.onNavyMuted, fontSize: 13 }}>{sub}</span>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 10 }}><Btn variant="ghostNavy" h={40}>Cancel</Btn><Btn variant="gold" h={40}>Print</Btn></div>
      </div>
      <div style={{ flexGrow: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', minHeight: 0 }}>{children}</div>
    </div>
  );
}

export function ReportCardScreen() {
  const subj = [['English', 78, 'B1'], ['Hindi', 84, 'A2'], ['Telugu', 71, 'B1'], ['Maths', 72, 'B1'], ['Science', 88, 'A2'], ['Social', 69, 'B2']];
  return (
    <PrintFrame title="Print report card" sub="A4 · Kavya Singh · Half Yearly 2026–27">
      <div data-hl="paper" style={{ width: 640, background: C.white, boxShadow: '0 30px 60px rgba(11,26,51,0.18)', padding: '30px 36px', display: 'flex', flexDirection: 'column', gap: 14, ...rise('vIn8', 0, 380) }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}><img src={A.logoLight} alt="" style={{ width: 140, height: 45 }} /><div style={{ textAlign: 'right' }}><span style={{ fontFamily: SERIF, fontSize: 20 }}>Saraswati Public School</span><Sub>12-2-823, Mehdipatnam, Hyderabad</Sub></div></div>
        <div style={{ borderTop: `2px solid ${C.navy}`, paddingTop: 12, display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}><span style={{ fontFamily: SERIF, fontSize: 28 }}>Progress report</span><span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 18, color: C.accent }}>Half Yearly 2026–27</span></div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 8, fontSize: 13 }}>{[['Student', 'Kavya Singh'], ['Class · Roll', 'VI-B · 14'], ['Attendance', '112 of 118 days']].map(([a, b]) => <div key={a}><Label>{a}</Label><div style={{ fontWeight: 600, marginTop: 3 }}>{b}</div></div>)}</div>
        <div data-hl="marks" style={{ border: `1px solid ${C.line}`, borderRadius: 8, overflow: 'hidden' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr 1fr', padding: '8px 14px', background: C.panel }}>{['Subject', 'Max', 'Marks', 'Grade'].map((h) => <Label key={h}>{h}</Label>)}</div>
          {subj.map(([n, m, g]) => <div key={n} style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr 1fr', padding: '7px 14px', borderTop: `1px solid ${C.track}`, fontSize: 13 }}><span>{n}</span><span>100</span><span style={{ fontWeight: 600 }}>{m}</span><span>{g}</span></div>)}
          <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr 1fr 1fr', padding: '9px 14px', borderTop: `1px solid ${C.line}`, background: C.bg, fontWeight: 600 }}><span>Total</span><span>600</span><span>462 · 77.0%</span><span>B1</span></div>
        </div>
        <div data-hl="remarks" style={{ borderRadius: 8, background: acc(0.06), border: `1px solid ${acc(0.2)}`, padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 4 }}><Label>Class teacher’s remarks</Label><span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 17, lineHeight: 1.4 }}>Kavya is attentive and asks good questions. More practice with maps will help in Social.</span></div>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: C.muted, marginTop: 14 }}><span>Class teacher ____________</span><span>Principal ____________</span><span>Parent ____________</span></div>
      </div>
    </PrintFrame>
  );
}

export function CalendarScreen() {
  const first = new Date(2026, 8, 1).getDay(); // weekday of 1 Sep 2026
  const events = { 5: ['event', "Teachers' Day"], 12: ['event', 'Parent–teacher meeting'], 21: ['exam', 'Half Yearly'], 22: ['exam', 'Half Yearly'], 23: ['exam', 'Half Yearly'], 24: ['exam', 'Half Yearly'], 25: ['exam', 'Half Yearly'], 28: ['holiday', 'School holiday'] };
  const tone = { event: [acc(0.12), C.accent], exam: [C.mkBg, C.mkFg], holiday: [C.unBg, C.unFg] };
  const cells = [];
  for (let i = 0; i < first; i++) cells.push(null);
  for (let d = 1; d <= 30; d++) cells.push(d);
  while (cells.length % 7) cells.push(null);
  return (
    <Desktop role="principal" active="calendar">
      <Main gap={20}>
        <PageTitle eyebrow="Academics · 2026–27" l1="School calendar." l2="Holidays, exams and events." actions={<><Btn variant="secondary" icon="chat">Share on WhatsApp</Btn><Btn icon="plus">Add event</Btn></>} />
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: 24, flexGrow: 1, minHeight: 0 }}>
          <section data-hl="month" style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, padding: '18px 22px', display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}><h2 style={{ margin: 0, fontFamily: SERIF, fontWeight: 400, fontSize: 28 }}>September 2026</h2><span style={{ fontSize: 13, color: C.muted }}>23 working days</span></div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 6 }}>
              {['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map((d) => <Label key={d}>{d}</Label>)}
              {cells.map((d, i) => {
                const ev = d && events[d]; const sun = i % 7 === 0; const today = d === 23;
                return (
                  <div key={i} data-hl={d === 28 ? 'holiday' : undefined} style={{ height: 96, borderRadius: 8, padding: 8, background: d ? (sun ? C.bg : C.white) : 'transparent', border: d ? `1px solid ${today ? C.accent : C.track}` : 0, boxShadow: today ? `0 0 0 3px ${acc(0.12)}` : 'none', display: 'flex', flexDirection: 'column', gap: 6 }}>
                    {d && <span style={{ fontFamily: SERIF, fontSize: 18, color: sun ? C.muted : C.ink }}>{d}</span>}
                    {ev && <span style={{ fontSize: 11, fontWeight: 600, padding: '3px 6px', borderRadius: 4, background: tone[ev[0]][0], color: tone[ev[0]][1], whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{ev[1]}</span>}
                  </div>
                );
              })}
            </div>
          </section>
          <Card eyebrow="Coming up" title="Next 3 weeks">
            {[['24–25 Sep', 'Half Yearly exams continue', 'exam'], ['28 Sep', 'School holiday', 'holiday'], ['3 Oct', 'Parent–teacher meeting', 'event'], ['9 Oct', 'Half Yearly results', 'event']].map(([d, t, k]) => (
              <div key={d} style={{ display: 'flex', gap: 12, padding: '13px 24px', borderTop: `1px solid ${C.track}` }}><span style={{ width: 8, height: 8, marginTop: 6, borderRadius: 4, background: tone[k][1], flexShrink: 0 }} /><div><b style={{ fontWeight: 500 }}>{t}</b><Sub>{d}</Sub></div></div>
            ))}
            <div style={{ margin: 'auto 24px 20px', fontSize: 12, color: C.muted, lineHeight: 1.5 }}>Holidays are not working days, so attendance percentages and fee due dates stay correct.</div>
          </Card>
        </div>
      </Main>
    </Desktop>
  );
}

/* ---- Circulars */
export function CircularsScreen({ state = 'compose' }) {
  const channels = [['Staff app', 'Inbox on every staff phone', true], ['Parent WhatsApp groups', 'Share to 13 class groups, one tap each', true], ['Email to parents', 'From the school’s Gmail', true], ['Printed notice', 'A4 letterhead with a tear-off slip', true], ['Automatic WhatsApp', 'Optional module · off', false]];
  return (
    <Desktop role="principal" active="circulars">
      <Main gap={22}>
        <PageTitle eyebrow="Communication · Circular 14 / 2026–27" l1={state === 'compose' ? 'New circular.' : 'Circular sent.'} l2={state === 'compose' ? 'Numbered and archived.' : 'Who has seen it.'} />
        {state === 'compose' ? (
          <div style={{ display: 'grid', gridTemplateColumns: '1.1fr 1fr', gap: 24 }}>
            <Card>
              <div style={{ padding: 24, display: 'flex', flexDirection: 'column', gap: 16 }}>
                <Field label="Title" value="Parent–teacher meeting on Saturday, 3 October" h={46} hl="title" />
                <Field label="Message" textarea h={110} value="Dear parents, the Half Yearly parent–teacher meeting is on Saturday, 3 October, 9 AM to 12 noon. Please meet your child’s class teacher. Report cards will be given at the meeting." />
                <div data-hl="audience" style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><span style={{ fontSize: 13, fontWeight: 500 }}>Send to</span><div style={{ display: 'flex', gap: 8 }}><Chip on>Whole school</Chip><Chip>Choose classes</Chip><Chip>Staff only</Chip></div></div>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}><span style={{ fontSize: 13, fontWeight: 500, marginRight: 6 }}>Languages</span>{[['English', true], ['हिंदी', false], ['తెలుగు', true]].map(([l, on]) => <Chip key={l} on={on}>{l}</Chip>)}</div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '10px 12px', borderRadius: 8, border: `1px dashed ${C.lineStrong}` }}><Icon name="attach" size={18} color={C.accent} />PTM-schedule.pdf<span style={{ marginLeft: 'auto', fontSize: 12, color: C.muted }}>120 KB</span></div>
              </div>
            </Card>
            <Card eyebrow="How to send" title="Channels" hl="channels">
              {channels.map(([t, d, on]) => (
                <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 14, padding: '13px 24px', borderTop: `1px solid ${C.track}`, opacity: on ? 1 : 0.55 }}>
                  <span style={{ width: 22, height: 22, borderRadius: 5, background: on ? C.accent : 'transparent', border: on ? 0 : `1.5px solid ${C.radioOff}`, color: C.white, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>{on && <Icon name="check" size={14} sw={2.4} />}</span>
                  <div style={{ flexGrow: 1 }}><b style={{ fontWeight: 500 }}>{t}</b><Sub>{d}</Sub></div>
                </div>
              ))}
              <div style={{ padding: '16px 24px 20px', marginTop: 'auto' }}><Btn full h={52} icon="arrow" hl="send">Send circular</Btn></div>
            </Card>
          </div>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.2fr', gap: 24 }}>
            <section data-hl="sentpanel" style={{ borderRadius: 16, background: GRAD.success, color: C.white, padding: 32, display: 'flex', flexDirection: 'column', gap: 16 }}>
              <div style={{ width: 64, height: 64, borderRadius: 32, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center', animation: `vPopFee 460ms ${EASE} both` }}><Icon name="circulars" size={28} sw={1.7} /></div>
              <TwoLine l1="Parent–teacher meeting." l2="Saturday, 3 October." size={34} color={C.white} italicColor={C.gold} track="-0.02em" as="span" />
              {[['WhatsApp', 'Shared to 13 class groups'], ['Email', '588 of 612 parents · 24 have no email'], ['Printed', '670 notices with tear-off slips'], ['Calendar', 'Added to 3 October']].map(([a, b]) => <div key={a} style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid rgba(255,255,255,0.14)', paddingTop: 10, fontSize: 14 }}><span style={{ color: C.onNavyMuted }}>{a}</span><span>{b}</span></div>)}
            </section>
            <Card eyebrow="Staff" title="Read by 12 of 15" hl="readlist">
              <div style={{ padding: '0 24px 14px' }}><div style={{ height: 8, borderRadius: 4, background: C.track, overflow: 'hidden' }}><div style={{ width: '80%', height: '100%', background: C.accent, borderRadius: 4, transformOrigin: 'left', animation: `vFill 800ms 200ms ${EASE} both` }} /></div></div>
              {[['Meena Iyer', 'Read 9:02 AM', true], ['Anita Rao', 'Read 9:10 AM', true], ['Suresh Patel', 'Read 9:15 AM', true], ['R. Nair', 'Not yet · on leave', false], ['S. Khan', 'Not yet', false], ['V. Lakshmi', 'Not yet', false]].map(([n, s, r]) => (
                <div key={n} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '11px 24px', borderTop: `1px solid ${C.track}` }}><span style={{ width: 8, height: 8, borderRadius: 4, background: r ? C.accent : C.gold }} /><b style={{ fontWeight: 500, flexGrow: 1 }}>{n}</b><span style={{ fontSize: 12, color: C.muted }}>{s}</span></div>
              ))}
              <div style={{ padding: '14px 24px 20px' }}><Btn variant="secondary" full h={44} icon="bell">Remind the 3 who haven’t read</Btn></div>
            </Card>
          </div>
        )}
      </Main>
    </Desktop>
  );
}

export function BackupsScreen() {
  const runs = [['Today, 6:02 AM', 'This PC + school Drive', '46 MB', 'Verified'], ['Yesterday, 6:01 AM', 'This PC + school Drive', '46 MB', 'Verified'], ['Mon 21 Sep, 6:03 AM', 'This PC + school Drive', '45 MB', 'Verified'], ['Sat 19 Sep, 6:00 AM', 'This PC only · no internet', '45 MB', 'Partial'], ['Fri 18 Sep, 6:02 AM', 'This PC + school Drive', '45 MB', 'Verified'], ['1 Sep, 6:01 AM', 'Monthly copy', '44 MB', 'Verified']];
  return (
    <Desktop role="principal" active="backups">
      <Main gap={22}>
        <PageTitle eyebrow="Safety" l1="Backups." l2="Checked every morning." actions={<Btn icon="backups">Back up now</Btn>} />
        <div data-hl="band" style={{ display: 'flex', alignItems: 'center', gap: 20, padding: '22px 26px', borderRadius: 16, background: C.navy, color: C.white }}>
          <span style={{ width: 52, height: 52, borderRadius: 26, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Icon name="shield" size={24} sw={1.7} /></span>
          <div style={{ flexGrow: 1 }}><span style={{ fontFamily: SERIF, fontSize: 28 }}>Verified today at 6:02 AM.</span><div style={{ fontSize: 14, color: C.onNavy, marginTop: 4 }}>Encrypted with your recovery key · on this PC and in the Principal’s Google Drive</div></div>
          <Btn variant="gold" h={44}>Copy to USB drive</Btn>
        </div>
        <Table cols="1.4fr 1.8fr 0.7fr 1fr" head={['When', 'Where', 'Size', 'Check']} rows={runs.map(([w, d, s, st]) => [w, d, s, <Pill key="p" tone={st === 'Verified' ? 'paid' : 'partpaid'}>{st}</Pill>])} />
        <span style={{ fontSize: 13, color: C.muted }}>Keeps 30 daily and 12 monthly copies in each place. A backup counts only after Vidya opens it and checks every record and the audit history.</span>
      </Main>
    </Desktop>
  );
}

export function StoreScreen() {
  const items = [['Class VI book set', 2450, 18], ['School shirt · size 28', 350, 42], ['School tie', 120, 60], ['Notebook pack (12)', 480, 4], ['Belt', 150, 35], ['ID card holder', 40, 120]];
  return (
    <Desktop role="accountant" active="fees" header={false}>
      <main style={{ padding: 48, display: 'flex', flexDirection: 'column', gap: 24, width: 780 }}>
        <PageTitle eyebrow="School store · optional module" l1="Books & uniforms." l2="Sold with stock counts." />
        <div data-hl="items" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 14 }}>
          {items.map(([n, p, st]) => (
            <div key={n} style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 14, padding: 16, display: 'flex', flexDirection: 'column', gap: 8 }}>
              <span style={{ fontWeight: 600 }}>{n}</span><span style={{ fontFamily: SERIF, fontSize: 24 }}>{money(p)}</span>
              <span style={{ fontSize: 12, color: st < 10 ? C.danger : C.muted, fontWeight: st < 10 ? 600 : 400 }}>{st < 10 ? `Only ${st} left` : `${st} in stock`}</span>
            </div>
          ))}
        </div>
      </main>
      <Overlay />
      <SheetPanel eyebrow="Store sale" title="Kavya Singh · VI-B" sub="Parent buying books and uniform">
        <div data-hl="cart" style={{ display: 'flex', flexDirection: 'column' }}>
          {[['Class VI book set', 1, 2450], ['School shirt · size 28', 2, 700], ['School tie', 1, 120]].map(([n, q, a]) => <div key={n} style={{ display: 'grid', gridTemplateColumns: '1fr 50px 90px', padding: '10px 0', borderBottom: `1px solid ${C.track}` }}><span>{n}</span><span style={{ color: C.muted }}>× {q}</span><span style={{ textAlign: 'right', fontWeight: 500 }}>{money(a)}</span></div>)}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', paddingTop: 12 }}><b style={{ fontWeight: 500 }}>Total</b><span style={{ fontFamily: SERIF, fontSize: 30 }}>₹3,270</span></div>
        </div>
        <Segmented options={['Cash', 'UPI', 'Cheque']} value="UPI" />
        <span style={{ fontSize: 13, color: C.muted }}>Stock goes down automatically. Store sales are kept separate from fees and appear in the cash book.</span>
        <div style={{ marginTop: 'auto' }}><Btn full h={52} icon="arrow" hl="record">Record sale · ₹3,270</Btn></div>
      </SheetPanel>
    </Desktop>
  );
}

function LeaveApproval({ done }) {
  const list = [['leave', 'Leave request', 'Meena Iyer · Casual leave · Thu 24 – Fri 25 Sep', 'Meena Iyer, Teacher', 'just now'], ...APPROVALS.filter((a) => a[0] !== 'attendance').slice(0, 3)];
  return (
    <Desktop role="principal" active="approvals" badge={done ? 3 : 4}>
      <Main gap={22}>
        <PageTitle eyebrow="4 waiting" l1="Approvals." l2="Leave, corrections, reversals." />
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 520px', gap: 24, flexGrow: 1, minHeight: 0 }}>
          <Card>
            {list.map(([tone, t, w, who, age], i) => (
              <div key={t} data-hl={i === 0 ? 'row' : undefined} style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '16px 24px', borderTop: `1px solid ${C.track}`, background: i === 0 ? acc(0.06) : 'transparent', boxShadow: i === 0 ? `inset 3px 0 0 ${C.accent}` : 'none' }}>
                <Pill tone={tone === 'leave' ? 'attendance' : tone} fixed>{t}</Pill>
                <div style={{ flexGrow: 1, minWidth: 0 }}><b style={{ fontWeight: 500, display: 'block', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{w}</b><Sub>{who} · {age}</Sub></div>
              </div>
            ))}
          </Card>
          {!done ? (
            <section data-hl="detail" style={{ background: C.surface, border: `1px solid ${C.line}`, borderRadius: 16, display: 'flex', flexDirection: 'column', overflow: 'hidden', ...rise('vIn8', 0, 300) }}>
              <div style={{ padding: '22px 24px 16px', display: 'flex', flexDirection: 'column', gap: 10, borderBottom: `1px solid ${C.track}` }}><Pill tone="attendance" fixed>Leave request</Pill><span style={{ fontFamily: SERIF, fontSize: 28 }}>Meena Iyer · 2 days</span><Sub>Casual leave · Thu 24 – Fri 25 Sep</Sub></div>
              <div style={{ padding: '18px 24px', display: 'flex', flexDirection: 'column', gap: 14, flexGrow: 1 }}>
                <div><Label>Reason</Label><p style={{ margin: '6px 0 0', fontSize: 15, lineHeight: 1.5 }}>“Family function in Vijayawada.”</p></div>
                <div data-hl="balance" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>{[['Casual leave left', '9 of 12'], ['Classes to cover', 'V-A attendance + 6 periods']].map(([a, b]) => <div key={a} style={{ padding: '12px 14px', borderRadius: 10, background: C.panel }}><Label>{a}</Label><div style={{ fontFamily: SERIF, fontSize: 20, marginTop: 4 }}>{b}</div></div>)}</div>
                <div style={{ borderRadius: 10, background: acc(0.08), padding: '12px 14px', fontSize: 13, color: C.accent }}>After approval, Vidya suggests substitutes from the timetable.</div>
              </div>
              <div style={{ background: C.panel, padding: '16px 24px 20px', display: 'flex', gap: 10 }}><Btn variant="secondary" h={50}>Return</Btn><Btn variant="secondary" h={50}>Reject</Btn><div style={{ flexGrow: 1 }}><Btn full h={50} icon="check" hl="approve">Approve leave</Btn></div></div>
            </section>
          ) : (
            <section data-hl="done" style={{ borderRadius: 16, background: GRAD.success, color: C.white, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 16, padding: 40, textAlign: 'center' }}>
              <div style={{ width: 76, height: 76, borderRadius: 38, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center', animation: `vPopFee 460ms ${EASE} both` }}><Icon name="check" size={34} sw={1.8} /></div>
              <div style={rise('vIn8', 120, 320)}><TwoLine l1="Leave approved." l2="Substitutes next." size={40} color={C.white} italicColor={C.gold} track="-0.02em" as="span" /></div>
              <span style={{ fontSize: 14, color: C.onNavy, ...rise('vIn8', 200, 320) }}>Meena Iyer · 24–25 Sep · salary register updated · Meena has been notified</span>
            </section>
          )}
        </div>
      </Main>
    </Desktop>
  );
}

/* ---- phone: absence alerts, homework & notes, staff day */
export function AbsenceScreen({ state = 'list' }) {
  const sent = state === 'sent';
  return (
    <Phone>
      <PhoneHeader title="Absent today." sub="V-A · Wed, 23 Sep" pill={<OnlinePill>Online</OnlinePill>}><Counts items={[[32, 'Present'], [2, 'Absent']]} /></PhoneHeader>
      <div style={{ flexGrow: 1, padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <span style={{ fontSize: 13, color: C.muted, lineHeight: 1.5 }}>Let parents know. Both options are free: email from the school, or WhatsApp with the message ready.</span>
        {[['Ananya Reddy', 'Srinivas Reddy'], ['Ishaan Khan', 'Farhan Khan']].map(([n, g], i) => (
          <div key={n} data-hl={i === 0 ? 'card' : undefined} style={{ borderRadius: 14, background: C.surface, border: `1px solid ${C.line}`, padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 10 }}>
            <div><b style={{ fontWeight: 600, fontSize: 15 }}>{n}</b><Sub>Guardian · {g}</Sub></div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
              {sent ? <span style={{ height: 40, borderRadius: 8, background: acc(0.1), color: C.accent, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6, fontSize: 13, fontWeight: 600, ...rise('vIn10', i * 120, 260) }}><Icon name="check" size={14} sw={2.2} />Email sent</span> : <Btn variant="secondary" h={40} icon="mail" style={{ fontSize: 13 }}>Email</Btn>}
              <Btn variant="secondary" h={40} icon="chat" style={{ fontSize: 13 }}>WhatsApp</Btn>
            </div>
          </div>
        ))}
        <div data-hl="msg" style={{ borderRadius: 12, background: C.panel, padding: '12px 14px', fontSize: 13, lineHeight: 1.5 }}><Label>Message</Label><div style={{ marginTop: 6 }}>“Dear parent, Ananya was absent from school today, Wed 23 Sep. Please inform the class teacher if she is unwell. — Saraswati Public School”</div></div>
        <div style={{ marginTop: 'auto' }}>{sent ? <Btn full h={52} variant="secondary" disabled>2 emails sent</Btn> : <Btn full h={52} icon="mail" hl="emailall">Email both parents</Btn>}</div>
      </div>
      {sent && <Toast bottom={90}>Sent from the school’s Gmail · free</Toast>}
    </Phone>
  );
}

export function NotesScreen({ state = 'compose' }) {
  return (
    <Phone>
      <PhoneHeader title="Homework & notes." sub="V-A · Maths" pill={<OnlinePill>Online</OnlinePill>} />
      <div style={{ flexGrow: 1, padding: 16, display: 'flex', flexDirection: 'column', gap: 14 }}>
        <div style={{ display: 'flex', gap: 8 }}><Chip on>Homework</Chip><Chip>Class notes</Chip></div>
        <Field textarea h={96} value="Chapter 5: Fractions. Do exercise 5.2, questions 1–10 in your notebook. Bring it on Friday." hl="text" focused={state === 'compose'} />
        <div data-hl="files" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <div style={{ height: 110, borderRadius: 12, background: '#2B3A33', padding: 12, display: 'flex', flexDirection: 'column', justifyContent: 'space-between', color: '#E7EFE9' }}>
            <span style={{ fontFamily: SERIF, fontStyle: 'italic', fontSize: 15 }}>3/4 = 6/8 = 9/12</span><span style={{ fontSize: 11, opacity: 0.8 }}>Blackboard photo · 210 KB</span>
          </div>
          <div style={{ height: 110, borderRadius: 12, background: C.white, border: `1px solid ${C.line}`, padding: 12, display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
            <Icon name="file" size={26} color={C.accent} /><span style={{ fontSize: 12 }}><b style={{ fontWeight: 600 }}>Fractions-worksheet.pdf</b><Sub>240 KB</Sub></span>
          </div>
        </div>
        <span style={{ fontSize: 12, color: C.muted }}>Photos are made smaller for mobile data. Study material only, no student photos or marks.</span>
        <div style={{ marginTop: 'auto' }}><Btn full h={52} icon="arrow" hl="share">Share with parents</Btn></div>
      </div>
      {state === 'share' && <div style={{ position: 'absolute', inset: 0, background: 'rgba(11,26,51,0.45)', animation: 'vFade 240ms ease both' }} />}
      {state === 'share' && (
        <div data-hl="sheet" style={{ position: 'absolute', left: 0, right: 0, bottom: 0, background: C.white, borderRadius: '22px 22px 0 0', padding: '14px 18px 26px', display: 'flex', flexDirection: 'column', gap: 16, animation: `vIn10 300ms ${EASE} both` }}>
          <span style={{ alignSelf: 'center', width: 40, height: 4, borderRadius: 2, background: C.line }} />
          <Label>Share to</Label>
          <div style={{ display: 'flex', gap: 18 }}>
            {[['WhatsApp', '#25A35A', 'chat'], ['Email', C.accent, 'mail'], ['Save in Vidya', C.navy, 'check']].map(([n, c, ic]) => <div key={n} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 6, fontSize: 12 }}><span style={{ width: 54, height: 54, borderRadius: 27, background: c, color: C.white, display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Icon name={ic} size={24} sw={1.8} /></span>{n}</div>)}
          </div>
          <Label>Recent groups</Label>
          {[['V-A Parents 2026–27', '34 parents'], ['V-A Maths doubts', '29 parents']].map(([g, n], i) => (
            <div key={g} data-hl={i === 0 ? 'group' : undefined} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '10px 12px', borderRadius: 10, background: i === 0 ? acc(0.08) : C.bg }}>
              <span style={{ width: 40, height: 40, borderRadius: 20, background: '#25A35A', color: C.white, display: 'flex', alignItems: 'center', justifyContent: 'center' }}><Icon name="students" size={18} /></span>
              <div style={{ flexGrow: 1 }}><b style={{ fontWeight: 600 }}>{g}</b><Sub>WhatsApp group · {n}</Sub></div>
            </div>
          ))}
        </div>
      )}
    </Phone>
  );
}

export function StaffDayScreen({ state = 'checkin' }) {
  if (state === 'inbox') return (
    <Phone>
      <PhoneHeader title="Inbox." sub="1 new circular" pill={<OnlinePill>Online</OnlinePill>} />
      <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <div data-hl="circular" style={{ borderRadius: 14, background: C.surface, border: `1.5px solid ${C.accent}`, padding: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>
          <span style={{ display: 'flex', justifyContent: 'space-between' }}><Pill tone="attendance">Circular 14 / 2026–27</Pill><span style={{ fontSize: 12, color: C.muted }}>9:00 AM</span></span>
          <span style={{ fontFamily: SERIF, fontSize: 22, lineHeight: 1.2 }}>Parent–teacher meeting on Saturday, 3 October</span>
          <span style={{ fontSize: 14, lineHeight: 1.5 }}>9 AM to 12 noon. Class teachers please have report cards ready by Friday.</span>
          <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: C.accent }}><Icon name="attach" size={15} />PTM-schedule.pdf</span>
          <Btn full h={46} icon="check" hl="read">Mark as read</Btn>
        </div>
      </div>
    </Phone>
  );
  if (state === 'leave' || state === 'leaveSent') return (
    <Phone>
      <PhoneHeader title="Apply for leave." sub="Meena Iyer · Teacher" pill={<OnlinePill>Online</OnlinePill>} />
      <div style={{ flexGrow: 1, padding: 16, display: 'flex', flexDirection: 'column', gap: 14 }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}><span style={{ fontSize: 13, fontWeight: 500 }}>Type</span><Segmented options={['Casual', 'Sick', 'Other']} value="Casual" /></div>
        <div data-hl="dates" style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>{[['From', 'Thu 24 Sep'], ['To', 'Fri 25 Sep']].map(([a, b]) => <div key={a} style={{ borderRadius: 12, border: `1px solid ${C.line}`, background: C.white, padding: '12px 14px' }}><Label>{a}</Label><div style={{ fontFamily: SERIF, fontSize: 22, marginTop: 4 }}>{b}</div></div>)}</div>
        <Field label="Reason" value="Family function in Vijayawada." h={46} />
        <div data-hl="balance" style={{ borderRadius: 12, background: C.panel, padding: '12px 14px', display: 'flex', justifyContent: 'space-between' }}><span>Casual leave left</span><b style={{ fontFamily: SERIF, fontWeight: 400, fontSize: 20 }}>9 of 12</b></div>
        <span style={{ fontSize: 12, color: C.muted }}>The Principal will arrange a substitute for V-A attendance.</span>
        <div style={{ marginTop: 'auto' }}>{state === 'leave' ? <Btn full h={52} icon="arrow" hl="send">Send leave request</Btn> : <Btn full h={52} variant="secondary" disabled>Sent · waiting for the Principal</Btn>}</div>
      </div>
      {state === 'leaveSent' && <Toast bottom={90}>Leave request sent to the Principal</Toast>}
    </Phone>
  );
  return (
    <Phone>
      <PhoneHeader title="My attendance." sub="Wed, 23 Sep" pill={<OnlinePill>Online</OnlinePill>}><Counts items={[[17, 'Present'], [1, 'Leave'], [0, 'Late']]} /></PhoneHeader>
      <div style={{ flexGrow: 1, padding: 16, display: 'flex', flexDirection: 'column', gap: 14 }}>
        <div data-hl="checkin" style={{ borderRadius: 16, background: C.navy, color: C.white, padding: 22, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, textAlign: 'center', ...rise('vIn10', 0, 300) }}>
          <span style={{ width: 60, height: 60, borderRadius: 30, border: '1px solid rgba(197,171,122,0.5)', color: C.gold, display: 'flex', alignItems: 'center', justifyContent: 'center', animation: 'vPopAtt 400ms ease both' }}><Icon name="login" size={26} sw={1.7} /></span>
          <span style={{ fontFamily: SERIF, fontSize: 26 }}>Checked in at 8:47 AM.</span>
          <span style={{ fontSize: 13, color: C.onNavyMuted }}>On school Wi-Fi · before the 9:00 AM bell</span>
        </div>
        <Card eyebrow="September" title="This month">
          {[['Mon 21 Sep', 'In 8:52 AM · Out 3:40 PM'], ['Tue 22 Sep', 'In 8:45 AM · Out 3:35 PM'], ['Fri 18 Sep', 'Casual leave · approved']].map(([d, s]) => <div key={d} style={{ display: 'flex', justifyContent: 'space-between', padding: '12px 20px', borderTop: `1px solid ${C.track}`, fontSize: 13 }}><b style={{ fontWeight: 500 }}>{d}</b><span style={{ color: C.muted }}>{s}</span></div>)}
        </Card>
        <div style={{ marginTop: 'auto' }}><Btn full h={52} variant="secondary" icon="calendar" hl="applyleave">Apply for leave</Btn></div>
      </div>
    </Phone>
  );
}


/* ================================================================== registry + gallery */
const initialMarks = NAMES.map((_, i) => (i >= 30 ? '' : i === 8 || i === 10 ? 'A' : 'P'));
const allP = NAMES.map(() => 'P');
const withA = (arr, ...idx) => arr.map((m, i) => (idx.includes(i) ? 'A' : m));
export const SCREENS = [
  { id: 'welcome', group: 'Getting started', title: 'Welcome', size: [1440, 960], C: WelcomeScreen, states: [{ label: 'Set up', props: { mode: 'setup' } }, { label: 'Join', props: { mode: 'join' } }, { label: 'Recover', props: { mode: 'recover' } }] },
  { id: 'activate', group: 'Getting started', title: 'Activate', size: [1440, 960], C: ActivateScreen, states: [{ label: 'Code entered', props: { state: 'entered' } }, { label: 'Checking', props: { state: 'checking' } }, { label: 'Active', props: { state: 'done' } }] },
  { id: 'setup', group: 'Getting started', title: 'Setup wizard', size: [1440, 960], C: SetupScreen, states: ['school', 'session', 'you', 'recovery', 'ready'].map((s) => ({ label: s, props: { step: s } })) },
  { id: 'pin', group: 'Getting started', title: 'PIN unlock', size: [1440, 960], C: PinScreen, states: [{ label: 'Entering', props: { filled: 4 } }] },
  { id: 'home', group: 'Principal', title: 'Home', size: [1440, 1080], C: PrincipalHome, states: [{ label: 'Today', props: {} }] },
  { id: 'staff', group: 'Principal', title: 'Staff & access', size: [1440, 1080], C: StaffScreen, states: [{ label: 'List', props: { panel: 'none' } }, { label: 'Add staff', props: { panel: 'add' } }, { label: 'Invitation', props: { panel: 'invite' } }] },
  { id: 'approvals', group: 'Principal', title: 'Approvals', size: [1440, 1080], C: ApprovalsScreen, states: [{ label: 'Review', props: { state: 'review' } }, { label: 'Approved', props: { state: 'approved' } }] },
  { id: 'fee', group: 'Accountant', title: 'Collect fee', size: [1440, 1080], C: CollectFeeScreen, states: [{ label: 'Enter', props: {} }, { label: 'Too much', props: { amount: '5000' } }, { label: 'Recorded', props: { done: true } }] },
  { id: 'receipt', group: 'Accountant', title: 'Print receipt', size: [1440, 1080], C: ReceiptScreen, states: [{ label: 'A5 preview', props: {} }] },
  { id: 'join', group: 'Teacher (phone)', title: 'Join school', size: [390, 844], C: JoinScreen, states: ['code', 'confirm', 'pin', 'syncing'].map((s) => ({ label: s, props: { step: s } })) },
  { id: 'thome', group: 'Teacher (phone)', title: 'Home', size: [390, 844], C: TeacherHome, states: [{ label: 'Today', props: { secondCard: 'marks', requestBadge: 0 } }, { label: 'Request approved', props: { secondCard: 'marks', requestBadge: 1 } }] },
  { id: 'attendance', group: 'Teacher (phone)', title: 'Attendance', size: [390, 844], C: AttendanceScreen, states: [
    { label: 'Start', props: { marks: initialMarks } }, { label: 'All present', props: { marks: allP } }, { label: '1 absent', props: { marks: withA(allP, 2) } },
    { label: '2 absent', props: { marks: withA(allP, 2, 5) } }, { label: 'Submitted', props: { marks: withA(allP, 2, 5), submitted: true } }] },
  { id: 'marks', group: 'Teacher (phone)', title: 'Marks', size: [390, 844], C: MarksScreen, states: [{ label: 'Entering', props: { state: 'entering' } }, { label: 'Complete', props: { state: 'complete' } }, { label: 'Submitted', props: { state: 'submitted' } }] },
  { id: 'correction', group: 'Teacher (phone)', title: 'Correction request', size: [390, 844], C: CorrectionScreen, states: [{ label: 'Form', props: { state: 'form' } }, { label: 'Sent', props: { state: 'sent' } }] },
  { id: 'requests', group: 'Teacher (phone)', title: 'My requests', size: [390, 844], C: RequestsScreen, states: [{ label: 'Waiting', props: { state: 'pending' } }, { label: 'Approved', props: { state: 'approved' } }] },
  { id: 'thomete', group: 'Teacher (phone)', title: 'Home in Telugu', size: [390, 844], C: TeacherHome, states: [{ label: 'తెలుగు', props: { secondCard: 'marks', requestBadge: 0, lang: 'te' } }] },
  { id: 'absence', group: 'Teacher (phone)', title: 'Absence alerts', size: [390, 844], C: AbsenceScreen, states: [{ label: 'Absent list', props: { state: 'list' } }, { label: 'Emails sent', props: { state: 'sent' } }] },
  { id: 'notes', group: 'Teacher (phone)', title: 'Homework & notes', size: [390, 844], C: NotesScreen, states: [{ label: 'Write', props: { state: 'compose' } }, { label: 'Share', props: { state: 'share' } }] },
  { id: 'staffday', group: 'Staff (phone)', title: 'Attendance, leave, inbox', size: [390, 844], C: StaffDayScreen, states: [{ label: 'Checked in', props: { state: 'checkin' } }, { label: 'Leave form', props: { state: 'leave' } }, { label: 'Leave sent', props: { state: 'leaveSent' } }, { label: 'Circular', props: { state: 'inbox' } }] },
  { id: 'settings', group: 'Principal', title: 'Settings', size: [1440, 1080], C: SettingsScreen, states: [{ label: 'UPI', props: { tab: 'payments' } }, { label: 'Google Drive', props: { tab: 'drive' } }, { label: 'Languages & modules', props: { tab: 'modules' } }] },
  { id: 'feesadmin', group: 'Principal', title: 'Fee structure & dues', size: [1440, 1080], C: FeesAdminScreen, states: [{ label: 'Instalments', props: { state: 'structure' } }, { label: 'Dues', props: { state: 'dues' } }, { label: 'Reminder', props: { state: 'reminder' } }] },
  { id: 'accounts', group: 'Principal', title: 'Accounts', size: [1440, 1080], C: AccountsScreen, states: [{ label: 'Cash book', props: { state: 'cashbook' } }, { label: 'Expense', props: { state: 'expense' } }, { label: 'Profit', props: { state: 'profit' } }] },
  { id: 'salary', group: 'Principal', title: 'Salary register', size: [1440, 1080], C: SalaryScreen, states: [{ label: 'September', props: {} }] },
  { id: 'timetable', group: 'Principal', title: 'Timetable & substitutes', size: [1440, 1080], C: TimetableScreen, states: [{ label: 'Week', props: { state: 'week' } }, { label: 'Substitute', props: { state: 'substitute' } }, { label: 'Assigned', props: { state: 'assigned' } }] },
  { id: 'exams', group: 'Principal', title: 'Exams', size: [1440, 1080], C: ExamsScreen, states: [{ label: 'Seating', props: { state: 'seating' } }, { label: 'Hall tickets', props: { state: 'hallticket' } }] },
  { id: 'reportcard', group: 'Principal', title: 'Report card', size: [1440, 1080], C: ReportCardScreen, states: [{ label: 'Print', props: {} }] },
  { id: 'calendar', group: 'Principal', title: 'Calendar', size: [1440, 1080], C: CalendarScreen, states: [{ label: 'September', props: {} }] },
  { id: 'circulars', group: 'Principal', title: 'Circulars', size: [1440, 1080], C: CircularsScreen, states: [{ label: 'Write', props: { state: 'compose' } }, { label: 'Sent', props: { state: 'sent' } }] },
  { id: 'leave', group: 'Principal', title: 'Leave approval', size: [1440, 1080], C: ApprovalsScreen, states: [{ label: 'Review', props: { state: 'leave' } }, { label: 'Approved', props: { state: 'leaveApproved' } }] },
  { id: 'backups', group: 'Principal', title: 'Backups', size: [1440, 1080], C: BackupsScreen, states: [{ label: 'Verified', props: {} }] },
  { id: 'store', group: 'Accountant', title: 'School store (optional)', size: [1440, 1080], C: StoreScreen, states: [{ label: 'Sale', props: {} }] },
];

export default function VidyaPrototype() {
  const bare = typeof window !== 'undefined' && /[?&]bare=1/.test(window.location.search);
  const [sel, setSel] = useState({ id: 'welcome', s: 0, n: 0 });
  useEffect(() => { window.__vidyaShow = (id, s) => setSel((p) => ({ id, s, n: p.n + 1 })); window.__vidyaReady = true; }, []);
  const scr = SCREENS.find((x) => x.id === sel.id) || SCREENS[0];
  const st = scr.states[Math.min(sel.s, scr.states.length - 1)];
  const Screen = scr.C;
  if (bare) return <><GlobalStyle /><div key={`${sel.id}-${sel.s}-${sel.n}`} style={{ width: scr.size[0], height: scr.size[1] }}><Screen {...st.props} /></div></>;
  const [w, h] = scr.size;
  const scale = Math.min(1, 1100 / w, 780 / h);
  const groups = [...new Set(SCREENS.map((x) => x.group))];
  return (
    <div style={{ display: 'flex', minHeight: '100vh', fontFamily: SANS, background: C.outer, color: C.ink }}>
      <GlobalStyle />
      <nav style={{ width: 260, background: C.navy, color: C.white, padding: '24px 16px', display: 'flex', flexDirection: 'column', gap: 18, flexShrink: 0 }}>
        <img src={A.logoDark} alt="Vidya Budget School" style={{ width: 160, height: 52 }} />
        {groups.map((g) => (
          <div key={g} style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <div style={{ fontSize: 10, fontWeight: 600, letterSpacing: '0.16em', textTransform: 'uppercase', color: C.onNavyLabel, padding: '0 10px 6px' }}>{g}</div>
            {SCREENS.filter((x) => x.group === g).map((x) => (
              <button key={x.id} type="button" onClick={() => setSel({ id: x.id, s: 0, n: sel.n + 1 })} style={{ textAlign: 'left', height: 34, padding: '0 10px', borderRadius: 6, border: 0, background: x.id === sel.id ? 'rgba(255,255,255,0.07)' : 'transparent', color: x.id === sel.id ? C.white : C.onNavy, fontSize: 14, cursor: 'pointer' }}>{x.title}</button>
            ))}
          </div>
        ))}
      </nav>
      <main style={{ flexGrow: 1, padding: 32, display: 'flex', flexDirection: 'column', gap: 16, alignItems: 'flex-start' }}>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          {scr.states.map((s, i) => <button key={s.label} type="button" onClick={() => setSel({ id: sel.id, s: i, n: sel.n + 1 })} style={{ height: 34, padding: '0 14px', borderRadius: 17, border: `1px solid ${i === sel.s ? C.navy : C.lineStrong}`, background: i === sel.s ? C.navy : 'transparent', color: i === sel.s ? C.white : C.ink, fontSize: 13, cursor: 'pointer' }}>{s.label}</button>)}
        </div>
        <div style={{ width: w * scale, height: h * scale, borderRadius: w < 500 ? 36 : 12, overflow: 'hidden', boxShadow: '0 20px 50px rgba(11,26,51,0.2)' }}>
          <div key={`${sel.id}-${sel.s}-${sel.n}`} style={{ width: w, height: h, transform: `scale(${scale})`, transformOrigin: 'top left' }}><Screen {...st.props} /></div>
        </div>
      </main>
    </div>
  );
}
