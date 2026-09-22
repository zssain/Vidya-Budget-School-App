// DEV-only page (#/__mocks) linking to the five mock files, served by the dev
// server at /design/screens/*.dc.html through the design-mock middleware.
const MOCKS: [string, string][] = [
  ['Welcome', '/design/screens/Welcome.dc.html'],
  ['Principal Home', '/design/screens/Main.dc.html'],
  ['Collect fee', '/design/screens/FeeCollection.dc.html'],
  ['Teacher Home', '/design/screens/TeacherHome.dc.html'],
  ['Attendance', '/design/screens/Attendance.dc.html'],
]

export default function Mocks() {
  return (
    <div style={{ padding: 40, fontFamily: "'Geist', system-ui, sans-serif", color: 'var(--ink)' }}>
      <h1 className="v-serif" style={{ fontFamily: "'Newsreader', Georgia, serif", fontWeight: 400 }}>
        Mock screens
      </h1>
      <p style={{ color: 'var(--muted)' }}>The unmodified design contract, rendered by design/runtime/support.js.</p>
      <ul style={{ lineHeight: 1.9 }}>
        {MOCKS.map(([name, href]) => (
          <li key={href}>
            <a href={href} style={{ color: 'var(--accent)' }}>
              {name}
            </a>{' '}
            <code style={{ color: 'var(--muted)' }}>{href}</code>
          </li>
        ))}
      </ul>
    </div>
  )
}
