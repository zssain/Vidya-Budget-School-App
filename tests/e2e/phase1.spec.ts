import { test, expect } from '@playwright/test'

// Interaction tests for the mock states (docs/01-MOCK-SPEC.md §8). These drive
// the real app through the DEV gallery single-screen routes.
const G = (screen: string, state?: string) =>
  `/#/__gallery?screen=${screen}${state ? `&state=${state}` : ''}`

test.describe('Collect fee', () => {
  test('typing 5000 shows the over-due error and disables the button', async ({ page }) => {
    await page.goto(G('collectFee', 'default'))
    await page.locator('#amt').fill('5000')
    await expect(page.getByText(/more than the ₹3,100 due/)).toBeVisible()
    await expect(page.getByRole('button', { name: 'Enter a valid amount' })).toBeDisabled()
  })

  test('Full due chip drives the balance to ₹0', async ({ page }) => {
    await page.goto(G('collectFee', 'default'))
    await page.getByRole('button', { name: /Full due/ }).click()
    // balance-after line proves pay = ₹3,100 (so balance = ₹0), button enabled
    await expect(page.getByText('₹3,100 due − ₹3,100 this payment')).toBeVisible()
    await expect(page.getByRole('button', { name: /Record ₹3,100 payment/ })).toBeEnabled()
  })

  test('Cash mode hides the reference field', async ({ page }) => {
    await page.goto(G('collectFee', 'default'))
    await expect(page.locator('#ref')).toBeVisible()
    await page.getByRole('radio', { name: 'Cash' }).click()
    await expect(page.locator('#ref')).toHaveCount(0)
  })
})

test.describe('Attendance', () => {
  test('submit locked until all marked; mark all / undo / submit', async ({ page }) => {
    await page.goto(G('attendance', 'default'))
    const submit = page.getByRole('button', { name: 'Submit attendance' })
    await expect(page.getByText('4 students not marked yet')).toBeVisible()
    await expect(submit).toBeDisabled()

    await page.getByRole('button', { name: 'Mark all present' }).click()
    await expect(submit).toBeEnabled()

    await page.getByRole('button', { name: 'Undo mark all' }).click()
    await expect(page.getByText('4 students not marked yet')).toBeVisible()
    await expect(submit).toBeDisabled()

    await page.getByRole('button', { name: 'Mark all present' }).click()
    await submit.click()
    await expect(page.getByText('Submitted, saved on this phone.')).toBeVisible()
    // rows locked after submit
    await expect(page.getByRole('button', { name: 'Aadhya Sharma present' })).toBeDisabled()
  })
})

test.describe('Welcome', () => {
  test('each radio swaps the field and the CTA', async ({ page }) => {
    await page.goto(G('welcome', 'setup'))
    await expect(page.getByRole('button', { name: 'Activate and continue' })).toBeVisible()
    await expect(page.getByText('Activation code', { exact: true })).toBeVisible()

    await page.getByRole('radio', { name: /Join my school/ }).click()
    await expect(page.getByRole('button', { name: 'Join school' })).toBeVisible()
    await expect(page.getByText('Invitation link or code', { exact: true })).toBeVisible()

    await page.getByRole('radio', { name: /Recover an existing school/ }).click()
    await expect(page.getByRole('button', { name: 'Start recovery' })).toBeVisible()
    await expect(page.getByText('Owner verification needed', { exact: true })).toBeVisible()
  })
})
