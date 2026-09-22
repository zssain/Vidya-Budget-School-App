import { test, expect, type Page } from '@playwright/test'

// Fidelity test (docs/01-MOCK-SPEC.md §9). For each screen/state we screenshot
// the unmodified mock (rendered by design/runtime/support.js) and the matching
// app gallery route at the same viewport and compare at ≤ 0.1%.
//
// Baseline flow: the FIRST screenshot in each test is the MOCK, so the committed
// baseline in tests/e2e/__screens__ is generated FROM THE MOCK; the SECOND is the
// APP, asserted against that mock baseline. On a fresh checkout run once to write
// the mock baselines (the first assertion writes + fails), then again to assert
// the app.

type Drive = (page: Page) => Promise<void>
const noop: Drive = async () => {}

interface Entry {
  name: string
  mock: string
  screen: string
  state?: string
  vp: { width: number; height: number }
  drive: Drive
}

const ENTRIES: Entry[] = [
  { name: 'welcome-setup', mock: 'Welcome.dc.html', screen: 'welcome', state: 'setup', vp: { width: 1440, height: 960 }, drive: noop },
  { name: 'welcome-join', mock: 'Welcome.dc.html', screen: 'welcome', state: 'join', vp: { width: 1440, height: 960 }, drive: (p) => p.getByRole('radio', { name: /Join my school/ }).click() },
  { name: 'welcome-recover', mock: 'Welcome.dc.html', screen: 'welcome', state: 'recover', vp: { width: 1440, height: 960 }, drive: (p) => p.getByRole('radio', { name: /Recover an existing school/ }).click() },
  { name: 'principal-home', mock: 'Main.dc.html', screen: 'principalHome', vp: { width: 1440, height: 1080 }, drive: noop },
  { name: 'collect-fee-default', mock: 'FeeCollection.dc.html', screen: 'collectFee', state: 'default', vp: { width: 1440, height: 1080 }, drive: noop },
  { name: 'collect-fee-over', mock: 'FeeCollection.dc.html', screen: 'collectFee', state: 'over', vp: { width: 1440, height: 1080 }, drive: (p) => p.locator('#amt').fill('5000') },
  { name: 'collect-fee-cash', mock: 'FeeCollection.dc.html', screen: 'collectFee', state: 'cash', vp: { width: 1440, height: 1080 }, drive: (p) => p.getByRole('radio', { name: 'Cash' }).click() },
  { name: 'collect-fee-success', mock: 'FeeCollection.dc.html', screen: 'collectFee', state: 'success', vp: { width: 1440, height: 1080 }, drive: (p) => p.getByRole('button', { name: /Record ₹1,000 payment/ }).click() },
  { name: 'teacher-home', mock: 'TeacherHome.dc.html', screen: 'teacherHome', vp: { width: 390, height: 844 }, drive: noop },
  { name: 'attendance-default', mock: 'Attendance.dc.html', screen: 'attendance', state: 'default', vp: { width: 390, height: 844 }, drive: noop },
  { name: 'attendance-markall', mock: 'Attendance.dc.html', screen: 'attendance', state: 'markall', vp: { width: 390, height: 844 }, drive: (p) => p.getByRole('button', { name: 'Mark all present' }).click() },
  {
    name: 'attendance-submitted',
    mock: 'Attendance.dc.html',
    screen: 'attendance',
    state: 'submitted',
    vp: { width: 390, height: 844 },
    drive: async (p) => {
      await p.getByRole('button', { name: 'Mark all present' }).click()
      await p.getByRole('button', { name: 'Submit attendance' }).click()
    },
  },
]

// Freeze animations to their settled end-state and hide the caret so both the
// mock and the app are deterministic before comparison.
async function freeze(page: Page) {
  await page.evaluate(async () => {
    try {
      await (document as unknown as { fonts?: { ready?: Promise<unknown> } }).fonts?.ready
    } catch {
      /* fonts API unavailable */
    }
  })
  await page.addStyleTag({
    content: '*{animation:none!important;transition:none!important;caret-color:transparent!important}',
  })
  await page.waitForTimeout(400)
}

for (const e of ENTRIES) {
  test(`fidelity: ${e.name}`, async ({ page }) => {
    await page.setViewportSize(e.vp)

    // 1) Mock (the baseline source).
    await page.goto(`/design/screens/${e.mock}`, { waitUntil: 'load' })
    await page.waitForTimeout(400) // let support.js render
    await e.drive(page)
    await freeze(page)
    expect(await page.screenshot()).toMatchSnapshot(`${e.name}.png`)

    // 2) App gallery route, asserted against the mock baseline.
    const q = e.state ? `&state=${e.state}` : ''
    await page.goto(`/#/__gallery?screen=${e.screen}${q}`, { waitUntil: 'load' })
    await freeze(page)
    expect(await page.screenshot()).toMatchSnapshot(`${e.name}.png`)
  })
}
