# VIDYA — MOCK SPECIFICATION (the visual contract)

The five files in `design/screens/` ARE the design. This document indexes what is in them
so nothing is missed. When this document and a mock file disagree, **the mock file wins**
— report the difference in the phase handoff.

## v2 amendments (Phase 11)

These override the matching parts of this document for v2:

- **Attendance is Present / Absent only.** The Leave (**L**) button, the Leave count and the
  Leave segment of the stacked bar are removed from the Attendance screen and the desktop
  register (owner-approved change to `design/screens/Attendance.dc.html`). The help text becomes
  "34 students · tap P or A". Legacy `L` marks already stored are shown in history as a muted
  "Leave (old)" pill. The `AttendanceRow` component keeps only the P and A buttons (§7); the
  Attendance fidelity baseline (§9) is regenerated. This supersedes the L references in §2.1
  (`--gold` "L button"), §5, §7 (AttendanceRow) and §8 (Attendance sample marks).
- **No attendance cut-off copy.** Principal Home's "Needs attention" no longer says "usually
  done by 10:30"; it reads "<class> attendance not submitted" with the sub-line "Class teacher
  <name> · not submitted yet today". There is no cut-off or low-attendance-threshold setting.
- **The prototype is the reference for every new screen.** Beyond these five mock screens, all
  new v2 screens follow `design/prototype/VidyaPrototype.jsx` (see `docs/03-PROTOTYPE-SPEC.md`),
  using the same tokens, components and animations catalogued here.

## 1. How to read a `.dc.html` mock file

| In the file | Meaning |
|---|---|
| `<helmet>` | Font links + global CSS + `@keyframes` for that screen |
| `style="…"` on elements | The exact visual values. Copy them; do not "clean them up" |
| `{{name}}` | Value computed in `renderVals()` at the bottom of the file |
| `<sc-for list="{{items}}" as="x">` | Loop |
| `<sc-if value="{{cond}}">` | Conditional block |
| `onClick="{{fn}}"`, `onChange="{{fn}}"` | Handlers defined in `renderVals()` |
| `<script type="text/x-dc" data-dc-script>` | Sample data (constructor + `renderVals()`) and interaction logic |
| `data-props` `accent` | User accent colour, default `#2F7479`, options `#2F7479 #1F4E8C #5B4B8A` |
| `<a href="Attendance.dc.html">` | Navigation between screens |
| `/_blob/<id>` | An image asset — see §4 |

## 2. Colour tokens (every colour used by the mock — no others allowed in `src/`)

### 2.1 Solid colours
| Token | Hex | Used for |
|---|---|---|
| `--bg` | `#F5F7F6` | App background (paper) |
| `--bg-welcome-outer` | `#E4ECEB` | Welcome outer frame (see note ‡) |
| `--surface` | `#FDFDFB` | Cards, sheet, inputs on paper |
| `--surface-welcome` | `#F6F8F7` | Welcome left panel |
| `--white` | `#FFFFFF` | Inputs, P/A/L off buttons, text on navy |
| `--panel` | `#E2EAEB` | Stat strip, card footer bands, sheet footer |
| `--track` | `#E8EDEC` | Row dividers, bar tracks |
| `--unmarked` | `#FAF4E6` | Unmarked attendance row |
| `--navy` | `#0C1B38` | Sidebar, phone header, dark cards, chips selected |
| `--navy-deep` | `#08152B` | (reserved; in token list) |
| `--navy-raised` | `#1C3358` | Avatars on navy |
| `--navy-glow` | `#1A3560` | Radial gradient highlight |
| `--ink` | `#13233F` | Body text on paper |
| `--muted` | `#56657A` | Secondary text |
| `--on-navy` | `#C9D2DE` | Nav text, body on navy |
| `--on-navy-muted` | `#9FACBF` | Secondary text on navy |
| `--on-navy-label` | `#8E9AAE` | Sidebar section labels |
| `--on-navy-strong` | `#E8EDF4` | Tile labels |
| `--line` | `#D5DDE0` | Card borders, header border |
| `--line-strong` | `#C9D3D2` | Secondary button borders, inputs, segmented |
| `--line-stat` | `#C9D6D5` | Stat strip dividers |
| `--radio-off` | `#B7C2C9` | Unselected radio ring |
| `--accent` | `#2F7479` (runtime) | Primary buttons, eyebrow, italic sub-lines |
| `--accent-hover` | `#245C60` | Link hover |
| `--accent-alt-1` / `-alt-2` | `#1F4E8C` / `#5B4B8A` | Accent options |
| `--gold` | `#C5AB7A` | Gold on navy, L button, today bar, badges |
| `--gold-text` | `#8C6A2F` | Small gold text on paper ("Not submitted") |
| `--gold-light` | `#E6D3A8` | Initials on navy |
| `--gold-line` | `#DCC495` | Unmarked row border |
| `--gold-on-navy-text` | `#E9D6AE` | Offline pill text on navy |
| `--danger` | `#C0392B` | Absent button, error ring, conflict dot |
| `--danger-soft` | `#D07A73` | Absent segment in stacked bar |
| `--online` | `#5FD0A0` | Online dot |
| `--online-text` | `#BFE8D6` | "Confirmed by school server" on navy |
| `--teal-soft` | `#7DB1B5` | Icons on navy, present segment |
| `--pill-partpaid-bg/fg` | `#F4ECDC` / `#6B5220` | Part paid, Student details badge |
| `--pill-unpaid-bg/fg` | `#F6E4E2` / `#8E2F2A` | Unpaid, Payment reversal badge, error text |
| `--pill-marks-bg/fg` | `#E7E3F1` / `#4A3B78` | Marks correction badge |
| Paid / Attendance badge | accent at 12% / accent | |

‡ `#E4ECEB` appears only in the Welcome root background. Keep it as its own token.

### 2.2 Transparent colours (copy exactly)
`rgba(255,255,255,0.04|0.05|0.07|0.08|0.10|0.12|0.14|0.3)` (surfaces/borders on navy) ·
`rgba(197,171,122,0.14|0.3|0.45|0.5|0)` (gold chip, pulse) ·
`rgba(125,177,181,0.55)` (fee chart bars) · `rgba(11,26,51,0.06|0.3|0.45)` (panel shadow,
sheet shadow, overlay) · `rgba(95,208,160,0.18|0.35)` (online halo, online pill border) ·
`rgba(192,57,43,0.14)` (error ring) · accent at `0.06 | 0.10 | 0.12` via the helper below.

### 2.3 Accent helper (replicate exactly)
```js
const n = parseInt(accent.slice(1), 16);
const rgba = (a) => `rgba(${(n>>16)&255},${(n>>8)&255},${n&255},${a})`;
// --accent-6 = rgba(0.06), --accent-10 = rgba(0.10), --accent-12 = rgba(0.12)
```
`lib/theme.ts setAccent(hex)` writes `--accent`, `--accent-6`, `--accent-10`,
`--accent-12` on `:root`.

### 2.4 Gradients (copy exactly)
- Welcome right panel: `radial-gradient(120% 80% at 80% 10%, #1A3560 0%, #0C1B38 55%, #08152B 100%)`
- Teacher Home root: `radial-gradient(110% 45% at 85% 0%, #1A3560 0%, #0C1B38 60%)`
- Attendance header: `radial-gradient(120% 90% at 90% 0%, #1A3560 0%, #0C1B38 65%)`
- Fee success: `radial-gradient(120% 70% at 50% 0%, #1A3560 0%, #0C1B38 60%)`

## 3. Typography

| Family | Use | Weights/styles in mock |
|---|---|---|
| Geist | All UI text | 400, 500, 600, 700 (700 only in badges/tile counts) |
| Newsreader | Headings, big numbers, italic sub-lines | 400, 400 italic (optical sizing ON) |
| Noto Sans Devanagari | Hindi (fallback in every stack) | 400, 500, 600 |

- The mock loads Newsreader with the optical-size axis (`opsz 6..72`) and browsers apply
  `font-optical-sizing: auto`. Big headings (46–64px) therefore use the display optical
  size. **Bundle the variable Newsreader with the opsz axis** (Fontsource variable package,
  `opsz` + `opsz-italic` files) so headings look identical. A static 400 file will NOT
  match.
- Stacks: UI `'Geist', 'Noto Sans Devanagari', system-ui, sans-serif`;
  serif `'Newsreader', Georgia, serif`.
- Font sizes used: 10, 11, 12, 13, 14, 15, 16, 19, 20, 22, 24, 26, 28, 30, 32, 34, 40, 46,
  60, 64 px. Use exactly these.
- Letter-spacing values: `-0.03em` (Welcome h1/h2), `-0.025em` (desktop h1), `-0.02em`
  (phone h1, stats, success), `-0.015em` (card h2, sheet name), `0.14em` (eyebrows 11px),
  `0.16em` (labels 10px).
- Numbers: `font-variant-numeric: tabular-nums` (tables, money), `lining-nums
  tabular-nums` on Newsreader numbers.
- Hindi: `:root[lang="hi"]` swaps Newsreader headings to Noto Sans Devanagari 500 at the
  same size and removes italics for Devanagari text.

## 4. Assets

| Mock reference | File | Where | Size in mock |
|---|---|---|---|
| `/_blob/c7724ba7cede6cc54dcf27ecbe1f94fd` | `design/assets/vidya-horizontal-on-dark.svg` | Desktop sidebars | 160×52 |
| same | same | Teacher Home header | 132×43 |
| `/_blob/018c620b68aa5806aa7ee412a1ecdefe` | `design/assets/vidya-horizontal-on-light.svg` | Welcome left panel | 190×62 |
| `/_blob/c7a72d1fc8dc4a2020b3cd64bb5ad940` | `design/assets/dwaar-mark-on-dark.svg` | Welcome right panel | 220×220 |

Hindi lockups (`design/brand-kit/svg/vidya-horizontal-hindi-on-dark|light.svg`) replace the
English lockups when the UI language is Hindi.
App icons: `design/brand-kit/windows/vidya.ico`, `macos/vidya.icns`, `android/res/*`
(adaptive icon, background `#0C1B38`), `png/*`. Use them; never regenerate icons.
Brand rules (`design/brand-kit/vidya-brand-guide.svg`): never recolour the door, stretch,
outline, add shadows, or place the logo on busy colour; lockup ≥ 120px wide on screen;
below 32px use the small icon.

## 5. Keyframes (per-screen variants — keep every variant; names are the app's names)

| App name | Definition | Screens |
|---|---|---|
| `vRise12` | `from{opacity:0;transform:translateY(12px)} to{opacity:1;transform:none}` | Principal Home |
| `vRise14` | same with `14px` | Welcome, Teacher Home |
| `vIn6` | `from{opacity:0;transform:translateY(6px)} to{opacity:1;transform:none}` | Welcome |
| `vIn8` | same with `8px` | Collect fee |
| `vIn10` | same with `10px` | Attendance |
| `vGrow` | `from{transform:scaleY(0)} to{transform:scaleY(1)}` | Home fee chart |
| `vFill` | `from{transform:scaleX(0)} to{transform:scaleX(1)}` | Home attendance bars |
| `vSheet` | `from{transform:translateX(40px);opacity:.3} to{transform:none;opacity:1}` | Collect fee sheet |
| `vFade` | `from{opacity:0} to{opacity:1}` | Collect fee overlay |
| `vPopFee` | `0%{transform:scale(.5);opacity:0} 60%{transform:scale(1.08);opacity:1} 100%{transform:scale(1)}` | Fee success |
| `vPopAtt` | `0%{transform:scale(.6);opacity:0} 60%{transform:scale(1.1);opacity:1} 100%{transform:scale(1)}` | Attendance submitted |
| `vPulse` | `0%,100%{box-shadow:0 0 0 0 rgba(197,171,122,0.45)} 50%{box-shadow:0 0 0 6px rgba(197,171,122,0)}` | Teacher Home due dot |

Easing everywhere it says so: `cubic-bezier(.2,.8,.2,1)`. Global button transition:
`background-color .18s ease, border-color .18s ease, color .18s ease, box-shadow .18s ease,
transform .1s ease`; `:active` scale `.985` (desktop), `.93` (Attendance P/A/L), `.96`
(Teacher Home tiles/buttons). `@media (prefers-reduced-motion: reduce){*{animation:none
!important;transition:none !important}}`.

### 5.1 Every animation usage (duration, delay)
- Welcome: left block `vRise14 520ms`; right heading `vRise14 600ms 120ms`; door mark block
  `vRise14 700ms 200ms`; field blocks `vIn6 260ms ease`.
- Principal Home: title `vRise12 520ms`; stat strip `+80ms`; approvals row `+160ms`;
  bottom row `+240ms`; attendance bars `vFill 800ms (300 + i·60)ms`; fee bars
  `vGrow 700ms (300 + i·70)ms`.
- Collect fee: overlay `vFade 240ms ease`; sheet `vSheet 380ms`; error text / reference
  block `vIn8 200–220ms ease`; success: circle `vPopFee 460ms`, texts `vIn8 320ms` at
  120/200/280/360ms.
- Teacher Home: greeting `vRise14 520ms`; due card `+80ms`; tiles `vRise14 480ms` at
  140, 180, 220 … 460ms; due dot `vPulse 2.2s ease-in-out infinite`.
- Attendance: Undo button `vIn10 200ms ease`; stacked bar segments `width .35s`; row
  colours `.18s`; submitted bar `vIn10 300ms` + icon `vPopAtt 400ms ease`; toast
  `vIn10 220ms`.

## 6. Icons (inline SVG, copied from the mock)

All icons are `viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round"
stroke-linejoin="round"`, stroke width 1.6 (sidebar, header), 1.75 (buttons), 1.3 (phone
tiles, two-tone teal `#7DB1B5` + gold `#C5AB7A`), 2–2.2 (checks). Copy every `<path>`,
`<rect>`, `<circle>` from the mock into `src/lib/icons.ts` under these names:
`home, approvals, students, attendance, marks, fees, daybook, staff, sync, backups,
settings, receipts, requests, search, bell, chevronDown, chevronRight, plus, arrowUpRight,
back, close, check, clock, help, shield, cloudOff, reportCard, myClasses, inbox, profile,
cloudSync`. Tile icons keep their two-tone split (which sub-paths are teal vs gold).
New screens reuse these icons only; a missing icon → ask.

## 7. Components (extracted from the mock; props in, no sample data inside)

Exact values are in the mock; the most important ones are listed so nothing is missed.

- **Sidebar** — 256px, navy, padding `26px 16px 20px`, gap 28. Logo 160×52 + school name
  12px `#9FACBF`. Section label 10px/600/.16em uppercase `#8E9AAE`, padding `0 10px 8px`.
  Item 38px, padding `0 10px`, radius 6, gap 11, 500, `#C9D2DE`, icon 18/1.6. Active: bg
  `rgba(255,255,255,0.07)`, white text, gold icon stroke, 5px gold dot at the end. Count
  badge: min 20×20, radius 10, gold bg, navy 11px/700. Server card: border
  `rgba(255,255,255,0.12)`, radius 10, padding `12px 14px`, dot 7px `#5FD0A0` + halo
  `0 0 0 3px rgba(95,208,160,0.18)`. User row: avatar 34, `#1C3358` / `#E6D3A8`, 13px/600.
- **Header** — 64px, bottom border `#D5DDE0`, padding `0 40px`, gap 14. Session button
  36px/radius 6. Search 400×38, radius 6, icon at left 12/top 11. SyncPill. Bell 38px
  circle with 7px gold dot.
- **PageTitle** — eyebrow 11px/600/.14em uppercase accent + 5px dot, gap 8; h1 Newsreader
  400 46px/1.05/-.025em; italic accent sub-line 46px/1.1; actions slot (44px buttons).
- **StatStrip** — grid 4, bg `#E2EAEB`, radius 16; cell padding `22px 24px`, gap 10,
  divider `#C9D6D5`; value Newsreader 40/1/-.02em; arrow 14; label 14; note 12 muted.
- **Card** — `#FDFDFB`, border `#D5DDE0`, radius 16; header padding `20px 24px 16px`;
  eyebrow 10px/.16em; title Newsreader 26/-.015em; link 13px with 1px underline + arrow.
  Variant `dark` (navy, title 22, gold italic total 28). Variant `footerBand` (`#E2EAEB`,
  padding `14px 24px`).
- **ListRow** — padding `14px 24px`, top border `#E8EDEC`, gap 16; title 500 ellipsis; sub
  12 muted; trailing action.
- **Badge** (fixed 164×26, radius 4, 12px/500) and **Pill** (auto, 24px, radius 4, padding
  0 10). Variants: partpaid, unpaid, paid, marks, attendance, details, reversal.
- **Button** — primary (accent, white, radius 6, arrow icon, `space-between` when full
  width), secondary (transparent, `#C9D3D2` border), ghost, navy-pill (success screens,
  radius 23), gold-pill. Heights 34 / 36 / 44 / 46 / 50 / 52. Disabled: `#C9D3D2` bg,
  `#56657A` text.
- **Chip** — 36px, radius 18, 13px/500; selected navy/white; else `#C9D3D2` border.
- **Segmented** — grid, border `#C9D3D2`, radius 6, 44px items, left dividers, selected
  accent/white.
- **MoneyInput** — 62px, radius 6, 1.5px accent border, ring `0 0 0 4px accent-12`, ₹ in
  Newsreader 28 muted, value Newsreader 30; error: `#C0392B` border + ring
  `rgba(192,57,43,0.14)` + 13px `#8E2F2A` message with `vIn8`.
- **TextField** — 46px (sheet) / 48px (Welcome, 1.5px accent + ring when focused), radius 6.
- **RadioCard** — padding `14px 16px`, radius 8, 1.5px border; selected accent border +
  accent-6 bg; dot 18px (selected: 5px accent border on white; else 1.5px `#B7C2C9`).
- **BarList**, **ColumnChart** (max bar 36px wide, height `v/55000·96`, radius `4 4 2 2`,
  today gold, others `rgba(125,177,181,0.55)`).
- **Sheet** — right panel 520px, inset 16px, radius 20, shadow `0 30px 60px
  rgba(11,26,51,0.3)`, overlay `rgba(11,26,51,0.45)`.
- **SuccessPanel** — navy radial gradient, 76px ring icon, two-line Newsreader 40 title.
- **PhoneHeader** — (a) logo variant: logo 132×43 + two 44px circular buttons; (b) page
  variant: navy gradient, back 44px, network pill (30px, radius 15, gold 14% bg, `#E9D6AE`
  text), Newsreader 32 title + italic gold 22 sub-line, 4-column counts, 4px stacked bar.
- **TaskCard** — rows with 10px dot (pulse on first), gold "Start" pill 38px.
- **Tile** — 112px tall, radius 16, bg `rgba(255,255,255,0.05)`, border
  `rgba(255,255,255,0.08)`, icon 34/1.3 two-tone, 13px label `#E8EDF4`, optional count
  badge 20px gold.
- **AttendanceRow** — padding `7px 7px 7px 14px`, radius 10; roll Newsreader 16 muted;
  name 15; three 44×44 radius 8 buttons (off: white, `#D5DDE0` border, muted). On: P
  accent/white, A `#C0392B`/white, L `#C5AB7A`/navy. Unmarked row `#FAF4E6` + `#DCC495`
  border. `aria-pressed`, `aria-label "{name} present|absent|on leave"`.
- **SyncPill** — variants confirmed (accent-10 bg, accent text, 6px dot), pending, offline,
  navy, online-on-navy.
- **Toast** — navy, radius 10, 14px, gold check, `role="status"`, auto-hide 2200ms, 108px
  above the bottom on phone.
- **Empty**, **Placeholder** ("Coming in a later phase" — must be gone by the last phase).

## 8. Screens, sample data and interaction logic

Sample data lives in each file's `<script type="text/x-dc">`. Phase 1 copies it VERBATIM
into `src/dev/fixtures/<screen>.ts`; later phases reproduce the same numbers from the demo
seed.

- **Welcome** — state `mode: 'setup'|'join'|'recover'`. Radio cards swap the field block
  (activation code / invitation link / owner-verification note) and the CTA ("Activate and
  continue" / "Join school" / "Start recovery"). "Sign in" link → PIN screen. Phone
  (width < 900): single column, navy panel hidden.
- **Principal Home** — stats, approvals (4), needs attention (2 + backup band), attendance
  by class (7, last "Not submitted"), fee collection 6 days (Thu 32,000 · Fri 41,000 · Sat
  28,500 · Mon 55,000 · Tue 46,200 · Today 48,500; total ₹2,51,200).
- **Collect fee** — state `{amount:'1000', mode:'upi', done:false}`; due 3100; amount =
  digits only, max 7; chips Full due ₹3,100 / ₹1,000 / ₹500; over due → error + disabled;
  reference label/hint per mode (UPI "UPI transaction ID" / "e.g. 426518903214"; Cheque
  "Cheque number and bank" / "e.g. 004512 · SBI"; hidden for Cash); balance after =
  max(due − amount, 0); button "Record ₹1,000 payment" / "Enter a valid amount"; success
  "Payment recorded. / Receipt R-A2-0419." with "Confirmed by school server"; "Collect
  another fee" resets.
- **Teacher Home** — static; "Start" and the Attendance tile open Attendance.
- **Attendance** — 34 students (names in constructor); initial marks: index ≥ 30 unmarked,
  8 and 10 A, 4 L, rest P. Tapping the active mark clears it. Counts + stacked bar. "Mark
  all present" stores the previous marks and shows "Undo mark all". "Save draft" → toast
  "Draft saved on this phone" (2200ms). Submit disabled while unmarked, with "{n} students
  not marked yet". Submitted → buttons disabled, navy bar "Submitted, saved on this
  phone.", pill "Offline · 1 waiting to send".

## 9. Fidelity test (how "indistinguishable from the mock" is proven)

1. **Mock renderer (dev only)**: Phase 1 writes `design/runtime/support.js`, a small
   script that renders `.dc.html` files in a normal browser: it implements `DCLogic`
   (props, state, setState), `{{path}}` holes, `<sc-for>`, `<sc-if>`, `on*` handlers,
   `<helmet>` (moved into `<head>`), and maps `/_blob/<id>` to `design/assets/` using §4.
   It is served only by the dev server and never shipped.
2. Playwright opens each mock through the renderer and the matching app route (gallery)
   at the same viewport, waits 1.5 s for animations, and compares screenshots with
   `toHaveScreenshot` (max diff 0.1%). Same for these states: Welcome setup/join/recover;
   Collect fee default / amount 5000 / Cash / success; Attendance default / after Mark all
   / submitted.
3. Owner also exports reference PNGs from the design canvas into `design/reference/`
   (names in `README-START-HERE.md`) for a manual side-by-side.
4. Any diff above threshold is a bug in the app, not in the mock.
