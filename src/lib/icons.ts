// Icon geometry copied verbatim from the five mocks in design/screens/*.dc.html.
// See docs/01-MOCK-SPEC.md §6. Do not invent geometry; every coordinate below is
// taken directly from an inline <svg> in a mock. All icons share the frame
// viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round"
// stroke-linejoin="round"; the stroke width is chosen at render time by <Icon>.
//
// Two-tone tile icons carry a per-part `stroke` (teal #7DB1B5 or gold #C5AB7A);
// mono icons omit `stroke` so they inherit the SVG's stroke (currentColor / color).

/** The teal used by the two-tone phone tiles (base shapes). */
export const ICON_TEAL = '#7DB1B5';
/** The gold used by the two-tone phone tiles (accent shapes). */
export const ICON_GOLD = '#C5AB7A';

/** One drawable part of an icon: a path, a rect, or a circle. */
export type IconPart =
  | { d: string; stroke?: string }
  | { rect: { x: number; y: number; width: number; height: number; rx?: number }; stroke?: string }
  | { circle: { cx: number; cy: number; r: number }; stroke?: string };

/** An icon definition is an ordered list of parts. */
export type IconDef = readonly IconPart[];

/** Every icon name allowed in src/ (docs/01-MOCK-SPEC.md §6). */
export type IconName =
  | 'home'
  | 'approvals'
  | 'students'
  | 'attendance'
  | 'marks'
  | 'fees'
  | 'daybook'
  | 'staff'
  | 'sync'
  | 'backups'
  | 'settings'
  | 'receipts'
  | 'requests'
  | 'search'
  | 'bell'
  | 'chevronDown'
  | 'chevronRight'
  | 'plus'
  | 'arrowUpRight'
  | 'back'
  | 'close'
  | 'check'
  | 'clock'
  | 'help'
  | 'shield'
  | 'cloudOff'
  | 'reportCard'
  | 'myClasses'
  | 'inbox'
  | 'profile'
  | 'cloudSync';

export const icons: Record<IconName, IconDef> = {
  // --- Sidebar + header (mono), from Main.dc.html --------------------------
  home: [{ d: 'M3 11.5 12 4l9 7.5' }, { d: 'M5 10v10h14V10' }],
  approvals: [
    { d: 'M9 11l3 3 8-8' },
    { d: 'M20 12v7a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h11' },
  ],
  students: [
    { circle: { cx: 9, cy: 8, r: 3.5 } },
    { d: 'M2.5 20a6.5 6.5 0 0 1 13 0' },
    { d: 'M16 4.5a3.5 3.5 0 0 1 0 7' },
    { d: 'M18 14a6 6 0 0 1 3.5 6' },
  ],
  attendance: [
    { rect: { x: 3.5, y: 5, width: 17, height: 15, rx: 2 } },
    { d: 'M8 3v4M16 3v4M3.5 10h17' },
  ],
  marks: [
    { rect: { x: 5, y: 4, width: 14, height: 17, rx: 2 } },
    { d: 'M9 4V3h6v1M9 11h6M9 15h4' },
  ],
  fees: [{ d: 'M7 5h10M7 9h10M13 20 7 13h2.5a4 4 0 0 0 0-8' }],
  daybook: [{ d: 'M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z' }, { d: 'M4 19V5' }],
  staff: [{ d: 'M12 3 5 6v5c0 4.5 3 8 7 10 4-2 7-5.5 7-10V6z' }],
  sync: [
    { d: 'M20 11a8 8 0 0 0-14.5-4.5L4 8' },
    { d: 'M4 3v5h5' },
    { d: 'M4 13a8 8 0 0 0 14.5 4.5L20 16' },
    { d: 'M20 21v-5h-5' },
  ],
  backups: [
    { rect: { x: 3, y: 4, width: 18, height: 5, rx: 1 } },
    { d: 'M5 9v10a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V9M10 13h4' },
  ],
  settings: [
    { d: 'M4 7h10M18 7h2M4 17h4M12 17h8' },
    { circle: { cx: 16, cy: 7, r: 2 } },
    { circle: { cx: 10, cy: 17, r: 2 } },
  ],
  search: [{ circle: { cx: 11, cy: 11, r: 7 } }, { d: 'm20 20-3.5-3.5' }],
  bell: [{ d: 'M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z' }, { d: 'M10 20a2 2 0 0 0 4 0' }],
  chevronDown: [{ d: 'm6 9 6 6 6-6' }],
  plus: [{ d: 'M12 5v14M5 12h14' }],
  arrowUpRight: [{ d: 'M7 17 17 7M8 7h9v9' }],

  // --- Finance sidebar (mono), from FeeCollection.dc.html ------------------
  receipts: [{ d: 'M6 3h12v18l-3-2-3 2-3-2-3 2z' }, { d: 'M9 8h6M9 12h6' }],
  requests: [{ d: 'M4 13h4l2 3h4l2-3h4' }, { d: 'M5 5h14l1 8v6H4v-6z' }],

  // --- Header + navigation (mono), from FeeCollection / TeacherHome --------
  chevronRight: [{ d: 'm9 6 6 6-6 6' }],
  close: [{ d: 'M6 6l12 12M18 6 6 18' }],

  // --- Attendance.dc.html (mono) -------------------------------------------
  back: [{ d: 'M19 12H5M11 6l-6 6 6 6' }],
  check: [{ d: 'm5 12.5 4.5 4.5L19 7.5' }],
  clock: [{ circle: { cx: 12, cy: 12, r: 9 } }, { d: 'M12 7v5l3 2' }],
  cloudOff: [
    { d: 'M3 3l18 18' },
    { d: 'M17.5 18H7a4.5 4.5 0 0 1-1.3-8.8M9 5.6A6 6 0 0 1 18 9a4 4 0 0 1 2.4 7.2' },
  ],

  // --- Welcome.dc.html (mono) ----------------------------------------------
  help: [
    { circle: { cx: 12, cy: 12, r: 9 } },
    { d: 'M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.3M12 17h.01' },
  ],
  // Welcome "Encrypted on your devices" shield (same geometry as sidebar staff).
  shield: [{ d: 'M12 3 5 6v5c0 4.5 3 8 7 10 4-2 7-5.5 7-10V6z' }],

  // --- Two-tone phone tiles (teal base + gold accent), from TeacherHome ----
  reportCard: [
    { d: 'M6 3h9l4 4v14H6z', stroke: ICON_TEAL },
    { d: 'M15 3v4h4', stroke: ICON_TEAL },
    { circle: { cx: 12, cy: 12.5, r: 2.5 }, stroke: ICON_GOLD },
    { d: 'm10.6 14.6-.8 3.4 2.2-1 2.2 1-.8-3.4', stroke: ICON_GOLD },
  ],
  myClasses: [
    { d: 'M3 10 12 4l9 6', stroke: ICON_TEAL },
    { d: 'M5 10v10h14V10', stroke: ICON_TEAL },
    { d: 'M10 20v-5h4v5', stroke: ICON_GOLD },
  ],
  inbox: [
    { d: 'M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z', stroke: ICON_TEAL },
    { d: 'M10 20a2 2 0 0 0 4 0', stroke: ICON_GOLD },
  ],
  profile: [
    { circle: { cx: 12, cy: 8, r: 4 }, stroke: ICON_TEAL },
    { d: 'M4 21a8 8 0 0 1 16 0', stroke: ICON_GOLD },
  ],
  // "Sync" phone tile: cloud (teal) + up arrow (gold).
  cloudSync: [
    { d: 'M7 18a4.5 4.5 0 0 1-.5-9 6 6 0 0 1 11.5 1.5A3.8 3.8 0 0 1 17.5 18z', stroke: ICON_TEAL },
    { d: 'm9.5 13.5 2.5-2.5 2.5 2.5M12 11v5', stroke: ICON_GOLD },
  ],
};
