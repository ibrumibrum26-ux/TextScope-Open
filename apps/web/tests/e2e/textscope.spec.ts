import { expect, test, type Page } from '@playwright/test'
import { Buffer } from 'node:buffer'
import { createDocx } from '../docx-fixture.ts'
import { createPdf } from '../pdf-fixture.ts'

interface RuntimeAudit {
  consoleErrors: string[]
  consoleWarnings: string[]
  failedRequests: string[]
  localRequests: string[]
  pageErrors: string[]
  remoteRequests: string[]
  unsuccessfulResponses: string[]
}

const applicationOrigin = 'http://127.0.0.1:4173'
const audits = new WeakMap<Page, RuntimeAudit>()
const observedLocalRequests = new Set<string>()
const observedWarnings = new Set<string>()

test.beforeEach(async ({ page }) => {
  const audit: RuntimeAudit = {
    consoleErrors: [],
    consoleWarnings: [],
    failedRequests: [],
    localRequests: [],
    pageErrors: [],
    remoteRequests: [],
    unsuccessfulResponses: [],
  }
  audits.set(page, audit)

  page.on('console', (message) => {
    const location = message.location()
    const source = location.url === '' ? '' : ` @ ${location.url}:${location.lineNumber}`
    if (message.type() === 'error') {
      audit.consoleErrors.push(`${message.text()}${source}`)
    } else if (message.type() === 'warning') {
      const warning = `${message.text()}${source}`
      audit.consoleWarnings.push(warning)
      observedWarnings.add(warning)
    }
  })
  page.on('pageerror', (error) => audit.pageErrors.push(error.message))
  page.on('request', (request) => {
    const url = new URL(request.url())
    if (url.protocol !== 'http:' && url.protocol !== 'https:') {
      return
    }

    const observation = `${request.method()} ${url.href} [${request.resourceType()}]`
    if (url.origin === applicationOrigin) {
      audit.localRequests.push(observation)
      observedLocalRequests.add(observation)
    } else {
      audit.remoteRequests.push(observation)
    }
  })
  page.on('requestfailed', (request) => {
    audit.failedRequests.push(`${request.method()} ${request.url()}: ${request.failure()?.errorText}`)
  })
  page.on('response', (response) => {
    const url = new URL(response.url())
    if (url.origin === applicationOrigin && response.status() >= 400) {
      audit.unsuccessfulResponses.push(`${response.status()} ${url.href}`)
    }
  })
})

test.afterEach(async ({ page }) => {
  const audit = audits.get(page)
  expect(audit, 'runtime audit must be installed before page navigation').toBeDefined()
  expect(audit?.pageErrors, 'the page must not emit uncaught JavaScript errors').toEqual([])
  expect(audit?.consoleErrors, 'the page console must not contain errors').toEqual([])
  expect(audit?.failedRequests, 'application resource requests must not fail').toEqual([])
  expect(audit?.unsuccessfulResponses, 'local application responses must be successful').toEqual([])
  expect(audit?.remoteRequests, 'the application must not make remote requests').toEqual([])
})

test.afterAll(() => {
  console.log('Observed local page requests:')
  for (const request of [...observedLocalRequests].sort()) {
    console.log(`  ${request}`)
  }

  if (observedWarnings.size === 0) {
    console.log('Observed browser console warnings: NONE')
  } else {
    console.log('Observed browser console warnings:')
    for (const warning of [...observedWarnings].sort()) {
      console.log(`  ${warning}`)
    }
  }
})

async function openApplication(page: Page): Promise<void> {
  await page.goto('/')
  await expect(page).toHaveTitle('TextScope')
  await expect(page.getByRole('heading', { level: 1, name: 'TextScope' })).toBeVisible()
}

async function analyze(page: Page, input: string): Promise<void> {
  await openApplication(page)
  await page.getByLabel('Text to analyze').fill(input)
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()
}

async function chooseCleanProfile(page: Page, label: string): Promise<void> {
  const profile = page.getByLabel('Clean profile')
  await expect(profile).toBeEnabled()
  await profile.selectOption({ label })
  await expect(profile).toHaveValue(
    label === 'Safe Clean'
      ? 'safe_clean'
      : label === 'Canonical Clean'
        ? 'canonical_clean'
        : 'pure_plain_text',
  )
}

async function cleanWithProfile(page: Page, input: string, profileLabel: string): Promise<void> {
  await openApplication(page)
  await page.getByLabel('Text to analyze').fill(input)
  await chooseCleanProfile(page, profileLabel)
  await page.getByRole('button', { name: 'Clean', exact: true }).click()
  await expect(page.getByTestId('clean-results')).toBeVisible()
}

function utf16FileBytes(text: string, byteOrder: 'le' | 'be'): Buffer {
  const littleEndianPayload = Buffer.from(text, 'utf16le')
  if (byteOrder === 'le') {
    return Buffer.concat([Buffer.from([0xff, 0xfe]), littleEndianPayload])
  }

  const bigEndianPayload = Buffer.from(littleEndianPayload)
  for (let index = 0; index < bigEndianPayload.length; index += 2) {
    const first = bigEndianPayload[index]
    const second = bigEndianPayload[index + 1]
    if (first === undefined || second === undefined) {
      throw new Error('Test fixture must contain complete UTF-16 code units.')
    }
    bigEndianPayload[index] = second
    bigEndianPayload[index + 1] = first
  }
  return Buffer.concat([Buffer.from([0xfe, 0xff]), bigEndianPayload])
}

async function selectLocalFile(
  page: Page,
  name: string,
  buffer: Buffer,
  mimeType = 'text/plain',
): Promise<void> {
  await page.getByLabel('Local text file', { exact: true }).check()
  await page.getByLabel('Choose TXT or Markdown file').setInputFiles({ name, mimeType, buffer })
}

async function selectHtmlFile(
  page: Page,
  name: string,
  source: string | Buffer,
  mimeType = 'text/html',
): Promise<void> {
  const buffer = typeof source === 'string' ? Buffer.from(source, 'utf8') : source
  await page.getByLabel('HTML file', { exact: true }).check()
  await page.getByLabel('Choose HTML file').setInputFiles({ name, mimeType, buffer })
}

async function selectDocxFile(
  page: Page,
  name: string,
  buffer: Buffer,
  mimeType = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
): Promise<void> {
  await page.getByLabel('DOCX file', { exact: true }).check()
  await page.getByLabel('Choose DOCX file').setInputFiles({ name, mimeType, buffer })
}

async function selectPdfFile(
  page: Page,
  name: string,
  buffer: Buffer,
  mimeType = 'application/pdf',
): Promise<void> {
  await page.getByLabel('PDF file', { exact: true }).check()
  await page.getByLabel('Choose PDF file').setInputFiles({ name, mimeType, buffer })
}

test('renders the application and initializes the real WASM module through the UI', async ({
  browser,
  page,
}) => {
  expect(browser.browserType().name()).toBe('chromium')
  console.log(`Browser runtime: Microsoft Edge ${browser.version()}`)
  await openApplication(page)
  await expect(page.getByLabel('Text to analyze')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Analyze' })).toBeEnabled()

  await page.getByLabel('Text to analyze').fill('Browser WASM initialization')
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()

  const audit = audits.get(page)
  expect(audit?.localRequests.some((request) => request.includes('.wasm'))).toBe(true)
})

test('reads rich clipboard representations explicitly through the real Edge permission', async ({ page }) => {
  await openApplication(page)
  await page.getByLabel('Rich Clipboard', { exact: true }).check()
  await page.evaluate(async () => {
    const item = new ClipboardItem({
      'text/plain': new Blob(['A😀B'], { type: 'text/plain' }),
      'text/html': new Blob([
        '<p>A😀B</p><img src="https://example.invalid/clipboard.png" onerror="window.__textscopeClipboardExecuted = true"><script>window.__textscopeClipboardExecuted = true</script>',
      ], { type: 'text/html' }),
    })
    await navigator.clipboard.write([item])
  })
  await page.getByRole('button', { name: 'Read Clipboard', exact: true }).click()
  await expect(page.getByTestId('clipboard-inspector')).toBeVisible()
  await expect(page.getByTestId('clipboard-inspector')).toContainText('text/plain')
  await expect(page.getByTestId('clipboard-inspector')).toContainText('text/html')
  await expect(page.getByTestId('clipboard-comparison-state')).toContainText('equivalent')
  await expect(page.getByTestId('clipboard-html-panel')).toContainText('A😀B')
  await expect(page.getByTestId('clipboard-provenance')).toContainText('unsupported')
  expect(await page.evaluate(() => (window as Window & { __textscopeClipboardExecuted?: boolean }).__textscopeClipboardExecuted)).toBeUndefined()

  await page.getByRole('button', { name: 'Analyze', exact: true }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()
  await page.getByRole('button', { name: 'Reveal', exact: true }).click()
  await expect(page.getByTestId('reveal-output')).toContainText('A😀B')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()
  await expect(page.getByTestId('clean-results')).toBeVisible()
})

test('rich clipboard fallback accepts a real paste event without DOM HTML execution', async ({ page }) => {
  await openApplication(page)
  await page.getByLabel('Rich Clipboard', { exact: true }).check()
  await page.evaluate(async () => {
    await navigator.clipboard.writeText('Fallback A😀B')
  })
  await page.getByLabel('Clipboard text preview and paste fallback').press('Control+V')
  await expect(page.getByTestId('clipboard-inspector')).toBeVisible()
  await expect(page.getByTestId('clipboard-plain-panel')).toContainText('Fallback A😀B')
})

test('the interface remains usable at desktop, tablet, and mobile widths', async ({ page }) => {
  for (const viewport of [{ width: 1280, height: 800 }, { width: 768, height: 900 }, { width: 390, height: 844 }]) {
    await page.setViewportSize(viewport)
    await openApplication(page)
    await expect(page.getByRole('heading', { level: 1, name: 'TextScope' })).toBeVisible()
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1)
    expect(overflow).toBe(false)
  }
})

test('preserves the A😀B scanner invariant across every supported input', async ({ page }) => {
  const expectScannerInvariant = async () => {
    await expect(page.getByTestId('analysis-results')).toBeVisible()
    await expect(
      page.getByRole('row').filter({ hasText: 'U+0042 LATIN CAPITAL LETTER B' }),
    ).toBeVisible()
  }

  await openApplication(page)
  await page.getByLabel('Text to analyze').fill('A😀B')
  await page.getByRole('button', { name: 'Analyze', exact: true }).click()
  await expectScannerInvariant()

  await openApplication(page)
  await page.getByLabel('Rich Clipboard', { exact: true }).check()
  await page.evaluate(async () => {
    await navigator.clipboard.writeText('A😀B')
  })
  await page.getByRole('button', { name: 'Read Clipboard', exact: true }).click()
  await page.getByRole('button', { name: 'Analyze', exact: true }).click()
  await expectScannerInvariant()

  await openApplication(page)
  await selectLocalFile(page, 'same.txt', Buffer.from('A😀B', 'utf8'))
  await page.getByRole('button', { name: 'Analyze', exact: true }).click()
  await expectScannerInvariant()

  await openApplication(page)
  await selectLocalFile(page, 'same.markdown', Buffer.from('A😀B', 'utf8'), 'text/markdown')
  await page.getByRole('button', { name: 'Analyze', exact: true }).click()
  await expectScannerInvariant()

  await openApplication(page)
  await selectHtmlFile(page, 'same.html', '<p>A😀B</p>')
  await page.getByRole('button', { name: 'Analyze', exact: true }).click()
  await expectScannerInvariant()

  await openApplication(page)
  await selectDocxFile(page, 'same.docx', createDocx({
    bodyXml: '<w:p><w:r><w:t>A😀B</w:t></w:r></w:p>',
  }))
  await page.getByRole('button', { name: 'Analyze', exact: true }).click()
  await expectScannerInvariant()

  await openApplication(page)
  await selectPdfFile(page, 'same.pdf', createPdf({ pages: ['A😀B'], unicode: true }))
  await page.getByRole('button', { name: 'Analyze', exact: true }).click()
  await expectScannerInvariant()
})

test('detects and reveals U+200B through the application boundary', async ({ page }) => {
  await analyze(page, '\u200B')

  await expect(page.getByTestId('invisible-count')).toHaveText('1')
  await expect(page.getByText('U+200B ZERO WIDTH SPACE')).toBeVisible()
  const findings = page.getByRole('region', { name: 'Technical findings' })
  await expect(findings).toContainText('invisible_character')
  await expect(findings).toContainText('informational')

  await page.getByRole('button', { name: 'Reveal' }).click()
  await expect(page.getByTestId('reveal-output')).toHaveText('⟦ZWSP⟧')
})

test('preserves Rust offsets across the A😀B boundary', async ({ page }) => {
  await analyze(page, 'A😀B')

  const bRow = page.getByRole('row').filter({ hasText: 'U+0042 LATIN CAPITAL LETTER B' })
  await expect(bRow).toBeVisible()
  await expect(bRow.locator('td').nth(0)).toHaveText('2')
  await expect(bRow.locator('td').nth(1)).toHaveText('3')
  await expect(bRow.locator('td').nth(2)).toHaveText('5')
})

test('shows the core bidi classification and finding for U+202E', async ({ page }) => {
  await analyze(page, '\u202E')

  await expect(page.getByTestId('bidi-controls-count')).toHaveText('1')
  const rloRow = page.getByRole('row').filter({ hasText: 'U+202E RIGHT-TO-LEFT OVERRIDE' })
  await expect(rloRow.locator('td').nth(4)).toHaveText('Cf')
  await expect(rloRow.locator('td').nth(5)).toHaveText('Common')
  const findings = page.getByRole('region', { name: 'Technical findings' })
  await expect(findings).toContainText('bidirectional_control')
  await expect(findings).toContainText('caution')
})

test('shows unusual whitespace returned by the core for U+00A0', async ({ page }) => {
  await analyze(page, '\u00A0')

  await expect(page.getByTestId('unusual-whitespace-count')).toHaveText('1')
  await expect(page.getByText('U+00A0 NO-BREAK SPACE')).toBeVisible()
  await expect(page.getByRole('region', { name: 'Technical findings' })).toContainText(
    'unusual_whitespace',
  )
})

test('preserves the conservative core observation for an emoji ZWJ sequence', async ({ page }) => {
  await analyze(page, '👩\u200D💻')

  await expect(page.getByTestId('invisible-count')).toHaveText('1')
  await expect(page.getByText('U+200D ZERO WIDTH JOINER')).toBeVisible()
  const findings = page.getByRole('region', { name: 'Technical findings' })
  await expect(findings).toContainText('invisible_character')
  await expect(findings).toContainText('informational')
  await expect(findings).toContainText(
    'Join control detected; it may carry legitimate linguistic or emoji semantics.',
  )
  await expect(findings).not.toContainText(/remov/i)
})

test('normal Portuguese text has no application-created findings', async ({ page }) => {
  await analyze(page, 'Olá, ação e coração. Tudo bem?')

  await expect(page.getByRole('region', { name: 'Technical findings' })).toContainText(
    'No technical findings.',
  )
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('empty input produces a valid result without a runtime error', async ({ page }) => {
  await analyze(page, '')

  await expect(page.getByTestId('characters-count')).toHaveText('0')
  await expect(page.getByText('The analyzed text is empty.')).toBeVisible()
  await expect(page.getByRole('region', { name: 'Technical findings' })).toContainText(
    'No technical findings.',
  )
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('normal text follows Analyze to Safe Clean with zero transformations', async ({ page }) => {
  const input = 'Olá, ação e coração. Tudo bem?'
  await analyze(page, input)

  const profile = page.getByLabel('Clean profile')
  await expect(profile).toBeEnabled()
  await expect(profile.locator('option')).toHaveCount(3)
  await expect(profile).toHaveValue('safe_clean')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()

  await expect(page.getByTestId('clean-text')).toHaveText(input)
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('0')
  await expect(page.getByTestId('verification-status')).toHaveText('verified')
  await expect(page.getByTestId('transformation-manifest')).toContainText(
    'No transformations were applied.',
  )
  await expect(page.getByLabel('Text to analyze')).toHaveValue(input)
})

test('Canonical Clean normalizes through Rust and displays its manifest', async ({ page }) => {
  const input = 'cafe\u0301'
  await cleanWithProfile(page, input, 'Canonical Clean')

  await expect(page.getByTestId('tcs-version')).toHaveText('TCS 0.1.0')
  await expect(page.getByTestId('clean-text')).toHaveText('café')
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('1')
  await expect(page.getByTestId('verification-status')).toHaveText('verified')
  await expect(page.getByTestId('content-preservation-status')).toHaveText('preserved')
  const transformation = page.getByTestId('transformation-entry')
  await expect(transformation).toContainText('TCS-NORMALIZATION-001')
  await expect(transformation).toContainText('normalize_nfc')
  await expect(transformation).toContainText('Original code points [3, 5)')
  await expect(page.getByLabel('Text to analyze')).toHaveValue(input)
})

test('Safe Clean preserves an emoji ZWJ sequence with a remaining finding', async ({ page }) => {
  const input = '👩\u200D💻'
  await cleanWithProfile(page, input, 'Safe Clean')

  await expect(page.getByTestId('clean-text')).toHaveText(input)
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('0')
  await expect(page.getByTestId('remaining-findings-count')).toHaveText('1')
  await expect(page.getByTestId('verification-status')).toHaveText(
    'verified_with_remaining_findings',
  )
  await expect(page.getByTestId('verification-report')).toContainText('invisible_character')
  await expect(page.getByTestId('verification-report')).toContainText(
    'legitimate linguistic or emoji semantics',
  )
})

test('NBSP is preserved by Safe Clean and explicit in Pure Plain Text', async ({ page }) => {
  const input = 'Olá\u00A0mundo'
  await cleanWithProfile(page, input, 'Safe Clean')

  await expect(page.getByTestId('clean-text')).toHaveText(input)
  await expect(page.getByTestId('remaining-findings-count')).toHaveText('1')
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('0')

  await chooseCleanProfile(page, 'Pure Plain Text')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()
  await expect(page.getByTestId('clean-text')).toHaveText('Olá mundo')
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('1')
  await expect(page.getByTestId('content-preservation-status')).toHaveText('needs_review')
  await expect(page.getByTestId('verification-status')).toHaveText('needs_review')
  await expect(page.getByTestId('transformation-entry')).toContainText('TCS-WHITESPACE-001')
})

test('Canonical Clean never makes a bidi control disappear silently', async ({ page }) => {
  const input = 'abc\u202Edef'
  await cleanWithProfile(page, input, 'Canonical Clean')

  await expect(page.getByTestId('clean-text')).toHaveText(input)
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('0')
  await expect(page.getByTestId('remaining-findings-count')).toHaveText('1')
  await expect(page.getByTestId('verification-report')).toContainText('bidirectional_control')
  await expect(page.getByTestId('verification-status')).toHaveText(
    'verified_with_remaining_findings',
  )
})

test('a canonical clean result produces no new transformations when reapplied', async ({ page }) => {
  await cleanWithProfile(page, '\uFEFFcafe\u0301\r\n', 'Canonical Clean')

  const cleanOutput = (await page.getByTestId('clean-text').textContent()) ?? ''
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('2')
  await page.getByLabel('Text to analyze').fill(cleanOutput)
  await chooseCleanProfile(page, 'Canonical Clean')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()

  await expect(page.getByTestId('clean-text')).toHaveText(cleanOutput)
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('0')
  await expect(page.getByTestId('verification-status')).toHaveText('verified')
})

test('Pure Plain Text output never overwrites the original textarea', async ({ page }) => {
  const input = 'Original\u00A0preservado'
  await cleanWithProfile(page, input, 'Pure Plain Text')

  await expect(page.getByTestId('clean-text')).toHaveText('Original preservado')
  await expect(page.getByLabel('Text to analyze')).toHaveValue(input)
  await expect(page.getByTestId('clean-results')).toContainText(
    'Original input was preserved separately.',
  )
})

test('ingests a UTF-8 TXT file locally and reports safe source metadata', async ({ page }) => {
  const input = 'Relatório local: ação e coração.'
  const bytes = Buffer.from(input, 'utf8')
  await openApplication(page)
  await selectLocalFile(page, 'relatório.txt', bytes, 'application/octet-stream')

  await expect(page.getByTestId('file-source-panel')).toBeVisible()
  await expect(page.getByTestId('file-name')).toHaveText('relatório.txt')
  await expect(page.getByTestId('source-kind')).toHaveText('TXT')
  await expect(page.getByTestId('raw-byte-length')).toHaveText(String(bytes.length))
  await expect(page.getByTestId('browser-mime')).toHaveText('application/octet-stream')
  await expect(page.getByTestId('source-encoding')).toHaveText('UTF-8')
  await expect(page.getByTestId('source-bom')).toHaveText('None')
  await expect(page.getByTestId('decode-status')).toHaveText('Decoded strictly')
  await expect(page.getByTestId('decoded-characters')).toHaveText(
    String(Array.from(input).length),
  )
  await expect(page.getByTestId('decoded-findings')).toHaveText('0')
  await expect(page.getByTestId('decoded-text')).toHaveText(input)
  await expect(page.getByTestId('source-unchanged')).toHaveText('Source file unchanged')

  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()
  await expect(page.getByRole('region', { name: 'Technical findings' })).toContainText(
    'No technical findings.',
  )
})

test('treats a UTF-8 BOM as source metadata, not decoded text', async ({ page }) => {
  const decoded = 'Olá'
  const bytes = Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), Buffer.from(decoded, 'utf8')])
  await openApplication(page)
  await selectLocalFile(page, 'bom.txt', bytes)

  await expect(page.getByTestId('source-encoding')).toHaveText('UTF-8')
  await expect(page.getByTestId('source-bom')).toHaveText('UTF-8 BOM')
  await expect(page.getByTestId('decoded-text')).toHaveText(decoded)
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('characters-count')).toHaveText('3')
  await expect(page.getByTestId('source-byte-range-0')).toHaveText('[3, 4)')
})

test('decodes UTF-16 LE and exposes source bytes separately from decoded offsets', async ({
  page,
}) => {
  await openApplication(page)
  await selectLocalFile(page, 'le.txt', utf16FileBytes('A😀B', 'le'))

  await expect(page.getByTestId('source-encoding')).toHaveText('UTF-16 little-endian')
  await expect(page.getByTestId('source-bom')).toHaveText('UTF-16 LE BOM')
  await expect(page.getByTestId('decoded-text')).toHaveText('A😀B')
  await page.getByRole('button', { name: 'Analyze' }).click()

  const bRow = page.getByRole('row').filter({ hasText: 'U+0042 LATIN CAPITAL LETTER B' })
  await expect(bRow.locator('td').nth(0)).toHaveText('2')
  await expect(bRow.locator('td').nth(1)).toHaveText('3')
  await expect(bRow.locator('td').nth(2)).toHaveText('5')
  await expect(page.getByTestId('source-byte-range-1')).toHaveText('[4, 8)')
})

test('decodes UTF-16 BE as literal Markdown text', async ({ page }) => {
  const decoded = '# Olá 😀'
  await openApplication(page)
  await selectLocalFile(page, 'notes.markdown', utf16FileBytes(decoded, 'be'), 'text/markdown')

  await expect(page.getByTestId('source-kind')).toHaveText('Markdown')
  await expect(page.getByTestId('source-encoding')).toHaveText('UTF-16 big-endian')
  await expect(page.getByTestId('source-bom')).toHaveText('UTF-16 BE BOM')
  await expect(page.getByTestId('decoded-text')).toHaveText(decoded)
  await expect(page.getByText('Markdown is displayed and analyzed as literal text.')).toBeVisible()
})

test('never renders Markdown or embedded HTML from a local file', async ({ page }) => {
  const decoded = '# Forensic heading\n<script>window.compromised = true</script>\n**literal**'
  await openApplication(page)
  await selectLocalFile(page, 'literal.md', Buffer.from(decoded, 'utf8'), 'text/markdown')

  await expect(page.getByTestId('decoded-text')).toHaveText(decoded)
  await expect(page.getByRole('heading', { name: 'Forensic heading' })).toHaveCount(0)
  await expect(page.locator('script')).toHaveCount(1)
  expect(await page.evaluate(() => 'compromised' in window)).toBe(false)
})

test('rejects invalid and ambiguous encodings with controlled errors and no stale result', async ({
  page,
}) => {
  await openApplication(page)
  await selectLocalFile(page, 'valid.txt', Buffer.from('valid', 'utf8'))
  await expect(page.getByTestId('file-source-panel')).toBeVisible()

  await page.getByLabel('Choose TXT or Markdown file').setInputFiles({
    name: 'invalid.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from([0xff]),
  })
  await expect(page.getByRole('alert')).toHaveText('The file is not strictly valid UTF-8.')
  await expect(page.getByTestId('file-source-panel')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Analyze' })).toBeDisabled()

  await page.getByLabel('Choose TXT or Markdown file').setInputFiles({
    name: 'ambiguous.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from([0x41, 0x00, 0x42, 0x00]),
  })
  await expect(page.getByRole('alert')).toHaveText(
    'The file resembles BOM-less UTF-16 and cannot be decoded without guessing.',
  )
  await expect(page.getByTestId('decoded-text')).toHaveCount(0)
})

test('rejects unsupported text extensions and files above the 10 MiB UI guard', async ({
  page,
}) => {
  await openApplication(page)
  await selectLocalFile(page, 'page.pdf', Buffer.from('private', 'utf8'), 'application/pdf')
  await expect(page.getByRole('alert')).toHaveText(
    'Only .txt, .md, and .markdown files are supported in this TextScope phase.',
  )

  await page.getByLabel('Choose TXT or Markdown file').setInputFiles({
    name: 'oversized.txt',
    mimeType: 'text/plain',
    buffer: Buffer.alloc(10 * 1024 * 1024 + 1),
  })
  await expect(page.getByRole('alert')).toHaveText(
    'The selected file exceeds the 10 MiB local interface limit.',
  )
  await expect(page.getByTestId('file-source-panel')).toHaveCount(0)
})

test('runs scanner and reveal on decoded file content with source provenance', async ({ page }) => {
  const decoded = 'A\u200BB'
  await openApplication(page)
  await selectLocalFile(page, 'finding.txt', Buffer.from(decoded, 'utf8'))
  await page.getByRole('button', { name: 'Analyze' }).click()

  await expect(page.getByTestId('invisible-count')).toHaveText('1')
  await expect(page.getByTestId('source-byte-range-1')).toHaveText('[1, 4)')
  await expect(page.getByRole('region', { name: 'Technical findings' })).toContainText(
    'invisible_character',
  )
  await page.getByRole('button', { name: 'Reveal' }).click()
  await expect(page.getByTestId('reveal-output')).toHaveText('A⟦ZWSP⟧B')
})

test('cleans decoded file text while preserving the source snapshot unchanged', async ({ page }) => {
  const decoded = 'Olá\u00A0mundo\r\n'
  await openApplication(page)
  await selectLocalFile(page, 'clean.txt', Buffer.from(decoded, 'utf8'))
  await chooseCleanProfile(page, 'Pure Plain Text')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()

  await expect(page.getByTestId('clean-results')).toBeVisible()
  expect(await page.getByTestId('decoded-text').textContent()).toBe(decoded)
  expect(await page.getByTestId('clean-text').textContent()).toBe('Olá mundo\n')
  await expect(page.getByTestId('source-unchanged')).toHaveText('Source file unchanged')
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('2')
  await expect(page.getByTestId('transformation-manifest')).toContainText(
    'TCS-LINE-ENDINGS-001',
  )
  await expect(page.getByTestId('transformation-manifest')).toContainText('TCS-WHITESPACE-001')
  await expect(page.getByTestId('verification-status')).toHaveText('needs_review')
})

test('supports empty files and resets file state on replacement or mode change', async ({ page }) => {
  await openApplication(page)
  await selectLocalFile(page, 'empty.txt', Buffer.alloc(0))

  await expect(page.getByTestId('decoded-text')).toHaveText('(empty text)')
  await expect(page.getByTestId('raw-byte-length')).toHaveText('0')
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('characters-count')).toHaveText('0')

  await page.getByLabel('Choose TXT or Markdown file').setInputFiles({
    name: 'replacement.md',
    mimeType: 'text/markdown',
    buffer: Buffer.from('replacement', 'utf8'),
  })
  await expect(page.getByTestId('file-name')).toHaveText('replacement.md')
  await expect(page.getByTestId('decoded-text')).toHaveText('replacement')
  await expect(page.getByTestId('analysis-results')).toHaveCount(0)

  await page.getByLabel('Paste text', { exact: true }).check()
  await expect(page.getByLabel('Text to analyze')).toHaveValue('')
  await expect(page.getByTestId('file-source-panel')).toHaveCount(0)
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('loads normal HTML and analyzes extracted document text through Rust', async ({ page }) => {
  const source =
    '<!doctype html><html><head><title>Sample</title></head><body><p>Hello HTML</p></body></html>'
  await openApplication(page)
  await selectHtmlFile(page, 'sample.html', source)

  await expect(page.getByTestId('html-source-panel')).toBeVisible()
  await expect(page.getByTestId('file-name')).toHaveText('sample.html')
  await expect(page.getByTestId('source-kind')).toHaveText('HTML')
  await expect(page.getByTestId('html-title')).toHaveText('Sample')
  await expect(page.getByTestId('extracted-document-text')).toHaveText('Hello HTML')
  await expect(page.getByRole('heading', { name: 'Hello HTML' })).toHaveCount(0)

  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()
  await expect(page.getByTestId('characters-count')).toHaveText('10')
})

test('supports .htm entities through scan, reveal, and clean', async ({ page }) => {
  const extracted = 'A\u00A0B\u200BC'
  await openApplication(page)
  await selectHtmlFile(page, 'entities.htm', '<p>A&nbsp;B&#x200B;C</p>')

  await expect(page.getByTestId('extracted-document-text')).toHaveText(extracted)
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('unusual-whitespace-count')).toHaveText('1')
  await expect(page.getByTestId('invisible-count')).toHaveText('1')

  await page.getByRole('button', { name: 'Reveal' }).click()
  await expect(page.getByTestId('reveal-output')).toHaveText('A⟦NBSP⟧B⟦ZWSP⟧C')

  await chooseCleanProfile(page, 'Pure Plain Text')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()
  await expect(page.getByTestId('clean-text')).toHaveText('A B\u200BC')
  await expect(page.getByTestId('transformation-entry')).toContainText('TCS-WHITESPACE-001')
  await expect(page.getByTestId('remaining-findings-count')).toHaveText('1')
})

test('never executes script source supplied by an HTML file', async ({ page }) => {
  const source =
    '<body>Safe<script>window.__TEXTSCOPE_TEST_EXECUTED__ = true;</script> text</body>'
  await openApplication(page)
  await selectHtmlFile(page, 'script.html', source)

  await expect(page.getByTestId('html-findings')).toContainText('active_script_present')
  await expect(page.getByTestId('html-active-content')).toContainText(
    'window.__TEXTSCOPE_TEST_EXECUTED__ = true;',
  )
  expect(await page.evaluate(() => '__TEXTSCOPE_TEST_EXECUTED__' in window)).toBe(false)
  await expect(page.locator('script')).toHaveCount(1)
})

test('never executes inline HTML event handlers', async ({ page }) => {
  const source = `<body><img src="missing.png" onerror="window.__TEXTSCOPE_EVENT_RAN__=true">
    <button onclick="window.__TEXTSCOPE_EVENT_RAN__=true">Label</button></body>`
  await openApplication(page)
  await selectHtmlFile(page, 'events.html', source)

  const findings = page.getByTestId('html-findings')
  await expect(findings).toContainText('inline_event_handler_present')
  expect(await page.evaluate(() => '__TEXTSCOPE_EVENT_RAN__' in window)).toBe(false)
  await expect(page.locator('img')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Label' })).toHaveCount(0)
})

test('detects an external resource while issuing zero request to its host', async ({ page }) => {
  const remoteUrl = 'https://example.invalid/tracker.png'
  await openApplication(page)
  await selectHtmlFile(page, 'reference.html', `<img src="${remoteUrl}"><p>Local text</p>`)

  await expect(page.getByTestId('external-references')).toContainText(remoteUrl)
  await expect(page.getByTestId('html-external-count')).toHaveText('1')
  const audit = audits.get(page)
  expect(audit?.remoteRequests).toEqual([])
})

test('displays title, language, meta, canonical, and base declarations', async ({ page }) => {
  const source = `<!doctype html><html lang="pt-BR" dir="ltr"><head>
    <title>Documento observado</title>
    <meta name="author" content="Alice">
    <meta property="og:title" content="Declaração">
    <link rel="canonical" href="https://example.invalid/canonical">
    <base href="/base/">
    </head><body>Texto</body></html>`
  await openApplication(page)
  await selectHtmlFile(page, 'metadata.html', source)

  await expect(page.getByTestId('html-title')).toHaveText('Documento observado')
  await expect(page.getByTestId('html-lang')).toHaveText('pt-BR')
  const metadata = page.getByTestId('html-metadata')
  await expect(metadata).toContainText('author=Alice')
  await expect(metadata).toContainText('og:title=Declaração')
  await expect(metadata).toContainText('canonical')
  await expect(metadata).toContainText('base href')
})

test('shows HTML comments separately from extracted document text', async ({ page }) => {
  await openApplication(page)
  await selectHtmlFile(page, 'comments.html', '<p>A</p><!-- internal note --><p>B</p>')

  await expect(page.getByTestId('html-comments')).toContainText('internal note')
  await expect(page.getByTestId('html-comments-count')).toHaveText('1')
  await expect(page.getByTestId('extracted-document-text')).toHaveText('A\nB')
  await expect(page.getByTestId('extracted-document-text')).not.toContainText('internal note')
})

test('keeps the application functional after malformed HTML recovery', async ({ page }) => {
  await openApplication(page)
  await selectHtmlFile(page, 'malformed.html', '<p>one<div>two</p></span>')

  await expect(page.getByTestId('html-findings')).toContainText('parser_recovery_observed')
  await expect(page.getByTestId('extracted-document-text')).not.toHaveText('(empty text)')
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('runs HTML bytes to parser to extracted text to TCS rescan and verification', async ({
  page,
}) => {
  await openApplication(page)
  await selectHtmlFile(page, 'pipeline.html', '<p>Olá&nbsp;mundo\r\n</p>')
  const originalExtracted = 'Olá\u00A0mundo\n'
  await expect(page.getByTestId('extracted-document-text')).toHaveText(originalExtracted)

  await chooseCleanProfile(page, 'Pure Plain Text')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()
  await expect(page.getByTestId('clean-text')).toHaveText('Olá mundo\n')
  await expect(page.getByTestId('verification-status')).toHaveText('needs_review')
  await expect(page.getByTestId('transformation-manifest')).toContainText(
    'TCS-WHITESPACE-001',
  )
  await expect(page.getByTestId('extracted-document-text')).toHaveText(originalExtracted)
})

test('preserves the HTML source snapshot across operations and mode reset', async ({ page }) => {
  const source = '<p>Original&nbsp;preserved</p>'
  await openApplication(page)
  await selectHtmlFile(page, 'original.html', source)

  await expect(page.getByTestId('source-unchanged')).toHaveText('Source file unchanged')
  await page.getByRole('button', { name: 'Analyze' }).click()
  await page.getByRole('button', { name: 'Reveal' }).click()
  await chooseCleanProfile(page, 'Pure Plain Text')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()
  await expect(page.getByTestId('source-unchanged')).toHaveText('Source file unchanged')
  await expect(page.getByTestId('extracted-document-text')).toHaveText(
    'Original\u00A0preserved',
  )

  await page.getByLabel('Paste text', { exact: true }).check()
  await expect(page.getByTestId('html-source-panel')).toHaveCount(0)
  await expect(page.getByLabel('Text to analyze')).toHaveValue('')
})

test('rejects invalid HTML mode extensions and oversized HTML before reading', async ({ page }) => {
  await openApplication(page)
  await selectHtmlFile(page, 'not-html.txt', 'private', 'text/plain')
  await expect(page.getByRole('alert')).toHaveText(
    'Only .html and .htm files are supported in HTML File mode.',
  )
  await expect(page.getByTestId('html-source-panel')).toHaveCount(0)

  await page.getByLabel('Choose HTML file').setInputFiles({
    name: 'oversized.html',
    mimeType: 'text/html',
    buffer: Buffer.alloc(10 * 1024 * 1024 + 1),
  })
  await expect(page.getByRole('alert')).toHaveText(
    'The selected file exceeds the 10 MiB local interface limit.',
  )
  await expect(page.getByTestId('html-source-panel')).toHaveCount(0)
})

test('loads a simple DOCX and exposes its deterministic package and body summary', async ({ page }) => {
  await openApplication(page)
  await selectDocxFile(page, 'simple.docx', createDocx({
    bodyXml: '<w:p><w:r><w:t>Hello DOCX</w:t></w:r></w:p>',
  }))

  await expect(page.getByTestId('docx-source-panel')).toBeVisible()
  await expect(page.getByTestId('file-name')).toHaveText('simple.docx')
  await expect(page.getByTestId('docx-package-status')).toHaveText('valid_wordprocessing_package')
  await expect(page.getByTestId('docx-parts')).toHaveText('3')
  await expect(page.getByTestId('docx-main-document')).toHaveText('Present')
  await expect(page.getByTestId('docx-paragraphs')).toHaveText('1')
  await expect(page.getByTestId('extracted-document-text')).toHaveText('Hello DOCX')

  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('characters-count')).toHaveText('10')
})

test('labels stored DOCX metadata as observed declarations rather than attribution', async ({ page }) => {
  const coreMetadata = '<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/"><dc:creator>Declared Alice</dc:creator><cp:lastModifiedBy>Declared Bob</cp:lastModifiedBy><dcterms:created>2026-01-02T03:04:05Z</dcterms:created></cp:coreProperties>'
  await openApplication(page)
  await selectDocxFile(page, 'metadata.docx', createDocx({
    extraEntries: [['docProps/core.xml', coreMetadata]],
  }))

  const metadata = page.getByTestId('docx-metadata')
  await expect(metadata).toContainText('Observed DOCX Metadata')
  await expect(metadata).toContainText('Stored declarations only; they are not verified attribution.')
  await expect(metadata).toContainText('Declared creator')
  await expect(metadata).toContainText('Declared Alice')
  await expect(metadata).toContainText('Declared last modified by')
  await expect(metadata).toContainText('Declared Bob')
})

test('detects an external DOCX relationship and performs zero remote request', async ({ page }) => {
  const remoteUrl = 'https://example.invalid/docx-resource.png'
  const relationship = `<Relationship Id="rExternal" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="${remoteUrl}" TargetMode="External"/>`
  await openApplication(page)
  await selectDocxFile(page, 'external.docx', createDocx({
    documentRelationships: relationship,
  }))

  await expect(page.getByTestId('docx-external-count')).toHaveText('1')
  await expect(page.getByTestId('docx-relationships')).toContainText(remoteUrl)
  await expect(page.getByTestId('docx-findings')).toContainText('external_relationship_present')
  expect(audits.get(page)?.remoteRequests).toEqual([])
})

test('keeps DOCX comments in a separate declared-metadata channel', async ({ page }) => {
  const comments = '<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:id="3" w:author="Declared Reviewer" w:initials="DR"><w:p><w:r><w:t>Review note</w:t></w:r></w:p></w:comment></w:comments>'
  await openApplication(page)
  await selectDocxFile(page, 'comments.docx', createDocx({
    bodyXml: '<w:p><w:r><w:t>Body only</w:t></w:r></w:p>',
    extraEntries: [['word/comments.xml', comments]],
  }))

  await expect(page.getByTestId('docx-comments-count')).toHaveText('1')
  await expect(page.getByTestId('docx-comments')).toContainText('Declared Reviewer')
  await expect(page.getByTestId('docx-comments')).toContainText('Review note')
  await expect(page.getByTestId('extracted-document-text')).toHaveText('Body only')
  await expect(page.getByTestId('extracted-document-text')).not.toContainText('Review note')
  await expect(page.getByTestId('docx-channels')).toContainText('Comments channel')
})

test('shows tracked insertion and deleted text without accepting or rejecting changes', async ({ page }) => {
  const body = '<w:p><w:r><w:t>Current </w:t></w:r><w:ins w:id="1" w:author="Declared Editor"><w:r><w:t>inserted</w:t></w:r></w:ins><w:del w:id="2" w:author="Declared Editor"><w:r><w:delText>deleted</w:delText></w:r></w:del></w:p>'
  await openApplication(page)
  await selectDocxFile(page, 'revisions.docx', createDocx({ bodyXml: body }))

  await expect(page.getByTestId('docx-revisions-count')).toHaveText('2')
  await expect(page.getByTestId('docx-revisions')).toContainText('inserted')
  await expect(page.getByTestId('docx-revisions')).toContainText('deleted')
  await expect(page.getByTestId('docx-revisions')).toContainText(
    'TextScope did not accept or reject any change.',
  )
  await expect(page.getByTestId('extracted-document-text')).toHaveText('Current inserted')
  await expect(page.getByTestId('extracted-document-text')).not.toContainText('deleted')
  await expect(page.getByTestId('docx-deleted-content')).toHaveText('deleted')
})

test('reports an explicitly hidden DOCX run while preserving its text', async ({ page }) => {
  const body = '<w:p><w:r><w:t>Visible </w:t></w:r><w:r><w:rPr><w:vanish/></w:rPr><w:t>hidden</w:t></w:r></w:p>'
  await openApplication(page)
  await selectDocxFile(page, 'hidden.docx', createDocx({ bodyXml: body }))

  await expect(page.getByTestId('docx-hidden-count')).toHaveText('1')
  await expect(page.getByTestId('docx-findings')).toContainText('explicit_hidden_text_present')
  await expect(page.getByTestId('extracted-document-text')).toHaveText('Visible hidden')
})

test('feeds DOCX NBSP, ZWSP, and emoji through scan and reveal', async ({ page }) => {
  const extracted = 'A\u00a0B\u200b😀'
  await openApplication(page)
  await selectDocxFile(page, 'unicode.docx', createDocx({
    bodyXml: `<w:p><w:r><w:t>${extracted}</w:t></w:r></w:p>`,
  }))

  await expect(page.getByTestId('extracted-document-text')).toHaveText(extracted)
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('unusual-whitespace-count')).toHaveText('1')
  await expect(page.getByTestId('invisible-count')).toHaveText('1')
  await page.getByRole('button', { name: 'Reveal' }).click()
  await expect(page.getByTestId('reveal-output')).toHaveText('A⟦NBSP⟧B⟦ZWSP⟧😀')
})

test('runs DOCX bytes through OOXML, extracted text, clean, rescan, and verification', async ({ page }) => {
  const extracted = 'Olá\u00a0mundo'
  await openApplication(page)
  await selectDocxFile(page, 'pipeline.docx', createDocx({
    bodyXml: `<w:p><w:r><w:t>${extracted}</w:t></w:r></w:p>`,
  }))
  await chooseCleanProfile(page, 'Pure Plain Text')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()

  await expect(page.getByTestId('clean-text')).toHaveText('Olá mundo')
  await expect(page.getByTestId('transformation-manifest')).toContainText('TCS-WHITESPACE-001')
  await expect(page.getByTestId('verification-status')).toHaveText('needs_review')
  await expect(page.getByTestId('clean-rescan-status')).toHaveText('Completed')
  await expect(page.getByTestId('extracted-document-text')).toHaveText(extracted)
})

test('returns a controlled error for malformed bytes named DOCX', async ({ page }) => {
  await openApplication(page)
  await selectDocxFile(page, 'malformed.docx', Buffer.from('not a ZIP package', 'utf8'))

  await expect(page.getByRole('alert')).toHaveText('The selected file is not a valid ZIP package.')
  await expect(page.getByTestId('docx-source-panel')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Analyze' })).toBeDisabled()
})

test('preserves the original DOCX bytes across analyze, reveal, clean, and reset', async ({ page }) => {
  const source = createDocx({
    bodyXml: '<w:p><w:r><w:t>Original\u00a0preserved</w:t></w:r></w:p>',
  })
  const immutableCopy = Buffer.from(source)
  await openApplication(page)
  await selectDocxFile(page, 'original.docx', source)

  await page.getByRole('button', { name: 'Analyze' }).click()
  await page.getByRole('button', { name: 'Reveal' }).click()
  await chooseCleanProfile(page, 'Pure Plain Text')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()
  await expect(page.getByTestId('source-unchanged')).toHaveText('Source file unchanged')
  await expect(page.getByTestId('extracted-document-text')).toHaveText('Original\u00a0preserved')
  expect(source.equals(immutableCopy)).toBe(true)

  await page.getByLabel('Paste text', { exact: true }).check()
  await expect(page.getByTestId('docx-source-panel')).toHaveCount(0)
  await expect(page.getByLabel('Text to analyze')).toHaveValue('')
})

test('inventories inert DOCX media, embedded objects, and custom XML as findings', async ({ page }) => {
  await openApplication(page)
  await selectDocxFile(page, 'inert-parts.docx', createDocx({
    extraEntries: [
      ['word/media/image1.png', Buffer.from([0x89, 0x50, 0x4e, 0x47])],
      ['word/embeddings/object1.bin', Buffer.from([1, 2, 3])],
      ['customXml/item1.xml', '<root>Observed only</root>'],
    ],
  }))

  await expect(page.getByTestId('docx-media-count')).toHaveText('1')
  await expect(page.getByTestId('docx-embedded-count')).toHaveText('1')
  await expect(page.getByTestId('docx-inert-parts')).toContainText('word/media/image1.png')
  await expect(page.getByTestId('docx-inert-parts')).toContainText('word/embeddings/object1.bin')
  await expect(page.getByTestId('docx-findings')).toContainText('embedded_object_present')
  await expect(page.getByTestId('docx-findings')).toContainText('custom_xml_present')
})

test('rejects DOCM and oversized DOCX before a package analysis can start', async ({ page }) => {
  await openApplication(page)
  await selectDocxFile(page, 'macro.docm', createDocx())
  await expect(page.getByRole('alert')).toHaveText(
    'Only .docx files are supported in DOCX File mode. Macro-enabled .docm files are not supported.',
  )
  await expect(page.getByTestId('docx-source-panel')).toHaveCount(0)

  await page.getByLabel('Choose DOCX file').setInputFiles({
    name: 'oversized.docx',
    mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    buffer: Buffer.alloc(10 * 1024 * 1024 + 1),
  })
  await expect(page.getByRole('alert')).toHaveText(
    'The selected file exceeds the 10 MiB local interface limit.',
  )
  await expect(page.getByTestId('docx-source-panel')).toHaveCount(0)
})

test('loads a simple PDF through the local Rust adapter and exposes forensic summary', async ({ page }) => {
  await openApplication(page)
  await selectPdfFile(page, 'simple.pdf', createPdf({ pages: ['Hello PDF'] }))

  await expect(page.getByTestId('pdf-source-panel')).toBeVisible()
  await expect(page.getByTestId('file-name')).toHaveText('simple.pdf')
  await expect(page.getByTestId('pdf-version')).toHaveText('1.7')
  await expect(page.getByTestId('pdf-pages')).toHaveText('1')
  await expect(page.getByTestId('pdf-extracted-text')).toHaveText('Hello PDF')
  await expect(page.getByText('Source file unchanged')).toBeVisible()

  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()
  await expect(page.getByTestId('characters-count')).toHaveText('9')
})

test('reports multipage PDF text, Info metadata, XMP, and page provenance without visual-order claims', async ({ page }) => {
  await openApplication(page)
  await selectPdfFile(page, 'metadata.pdf', createPdf({ pages: ['First page', 'Second page'], metadata: true, xmp: true }))

  await expect(page.getByTestId('pdf-pages')).toHaveText('2')
  await expect(page.getByTestId('pdf-extracted-text')).toHaveText('First page\n\nSecond page')
  await expect(page.getByTestId('pdf-metadata')).toContainText('Declared title')
  await expect(page.getByTestId('pdf-metadata')).toContainText('well-formed XML')
  await expect(page.getByTestId('pdf-source-panel')).toContainText('visual reading order is not guaranteed')
})

test('preserves representable PDF Unicode through extraction and the scanner pipeline', async ({ page }) => {
  const sourceText = 'Olá PDF'
  await openApplication(page)
  await selectPdfFile(page, 'unicode.pdf', createPdf({ pages: [sourceText] }))

  await expect(page.getByTestId('pdf-extracted-text')).toHaveText(sourceText)
  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()
  await page.getByRole('button', { name: 'Reveal' }).click()
  await expect(page.getByTestId('reveal-output')).toHaveText(sourceText)
  expect(audits.get(page)?.remoteRequests).toEqual([])
})

test('inventories PDF JavaScript and URI actions without executing or requesting them', async ({ page }) => {
  await openApplication(page)
  await selectPdfFile(page, 'active.pdf', createPdf({ javascript: true, uri: true }))

  await expect(page.getByRole('heading', { name: 'Actions and JavaScript' })).toBeVisible()
  await expect(page.getByTestId('pdf-source-panel')).toContainText('javascript')
  await expect(page.getByTestId('pdf-source-panel')).toContainText('https://example.invalid/textscope')
  expect(await page.evaluate(() => 'app' in window)).toBe(false)
  expect(audits.get(page)?.remoteRequests).toEqual([])
})

test('reports PDF attachments, forms, XFA, and signature structure without opening embedded bytes', async ({ page }) => {
  await openApplication(page)
  await selectPdfFile(page, 'evidence.pdf', createPdf({ attachment: true, form: true, signature: true }))

  await expect(page.getByRole('heading', { name: 'Embedded files' })).toBeVisible()
  await expect(page.getByTestId('pdf-source-panel')).toContainText('evidence.txt')
  await expect(page.getByTestId('pdf-source-panel')).toContainText('Forms / XFA')
  await expect(page.getByTestId('pdf-source-panel')).toContainText('Signature structures')
  await expect(page.getByTestId('pdf-source-panel')).toContainText('ByteRange')
  expect(audits.get(page)?.remoteRequests).toEqual([])
})

test('marks a PDF with no text operators and does not claim OCR text', async ({ page }) => {
  await openApplication(page)
  await selectPdfFile(page, 'image-only.pdf', createPdf({ pages: [''] }))

  await expect(page.getByTestId('pdf-text-status')).toHaveText('no_text_operators')
  await expect(page.getByTestId('pdf-extracted-text')).toHaveText('(no extracted text)')
  await expect(page.getByTestId('pdf-source-panel')).toContainText('OCR is not implemented')
  await expect(page.getByTestId('pdf-source-panel')).toContainText('no_text_layer')
})

test('runs PDF extracted text through Analyze, Reveal, and Clean while preserving the source snapshot', async ({ page }) => {
  const sourceText = 'Ola PDF'
  await openApplication(page)
  await selectPdfFile(page, 'pipeline.pdf', createPdf({ pages: [sourceText] }))

  await page.getByRole('button', { name: 'Analyze' }).click()
  await expect(page.getByTestId('analysis-results')).toBeVisible()
  await page.getByRole('button', { name: 'Reveal' }).click()
  await expect(page.getByTestId('reveal-output')).toHaveText(sourceText)
  await chooseCleanProfile(page, 'Safe Clean')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()

  await expect(page.getByTestId('clean-text')).toHaveText(sourceText)
  await expect(page.getByTestId('clean-transformations-count')).toHaveText('0')
  await expect(page.getByTestId('source-unchanged')).toHaveText('Source file unchanged')
  await expect(page.getByTestId('pdf-extracted-text')).toHaveText(sourceText)
})

test('returns a controlled error for malformed bytes named PDF and clears stale analysis', async ({ page }) => {
  await openApplication(page)
  await selectPdfFile(page, 'malformed.pdf', Buffer.from('not a PDF', 'utf8'))

  await expect(page.getByRole('alert')).toHaveText('The selected bytes do not contain a supported PDF header.')
  await expect(page.getByTestId('pdf-source-panel')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Analyze' })).toBeDisabled()
})

test('rejects non-PDF extensions and files above the PDF 10 MiB guard before reading', async ({ page }) => {
  await openApplication(page)
  await selectPdfFile(page, 'wrong.txt', Buffer.from('private', 'utf8'), 'text/plain')
  await expect(page.getByRole('alert')).toHaveText('Only .pdf files are supported in PDF File mode.')

  await page.getByLabel('Choose PDF file').setInputFiles({
    name: 'oversized.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.alloc(10 * 1024 * 1024 + 1),
  })
  await expect(page.getByRole('alert')).toHaveText('The selected file exceeds the 10 MiB local interface limit.')
  await expect(page.getByTestId('pdf-source-panel')).toHaveCount(0)
})

test('keeps original PDF bytes represented by the immutable upload buffer across operations and reset', async ({ page }) => {
  const source = createPdf({ pages: ['Original PDF'] })
  const immutableCopy = Buffer.from(source)
  await openApplication(page)
  await selectPdfFile(page, 'original.pdf', source)
  await page.getByRole('button', { name: 'Analyze' }).click()
  await page.getByRole('button', { name: 'Reveal' }).click()
  await chooseCleanProfile(page, 'Safe Clean')
  await page.getByRole('button', { name: 'Clean', exact: true }).click()
  await expect(page.getByTestId('source-unchanged')).toHaveText('Source file unchanged')
  expect(source.equals(immutableCopy)).toBe(true)

  await page.getByLabel('Paste text', { exact: true }).check()
  await expect(page.getByTestId('pdf-source-panel')).toHaveCount(0)
  await expect(page.getByLabel('Text to analyze')).toHaveValue('')
})
