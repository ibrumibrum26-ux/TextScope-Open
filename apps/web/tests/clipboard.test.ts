import assert from 'node:assert/strict'
import test from 'node:test'

import {
  MAX_CLIPBOARD_BYTES,
  ClipboardReadError,
  clipboardPayloadFromStrings,
  readClipboard,
} from '../src/clipboard.ts'

test('plain-text clipboard payload preserves Unicode bytes and MIME', () => {
  const payload = clipboardPayloadFromStrings('A😀B', null, [])
  assert.equal(payload.plain_text, 'A😀B')
  assert.equal(payload.representations[0]?.mime_type, 'text/plain')
  assert.equal(payload.representations[0]?.byte_length, new TextEncoder().encode('A😀B').byteLength)
})

test('HTML-only clipboard payload preserves source and advertises unsupported MIME', () => {
  const payload = clipboardPayloadFromStrings(null, '<p>A&nbsp;😀B</p><script>window.__bad=1</script>', ['application/rtf'])
  assert.match(payload.html_source ?? '', /script/)
  assert.deepEqual(payload.representations.map((item) => item.mime_type), ['text/html', 'application/rtf'])
})

test('both representations remain present for deterministic comparison', () => {
  const payload = clipboardPayloadFromStrings('A😀B', 'A😀B', [])
  assert.equal(payload.representations.length, 2)
})

test('line-ending differences are technical and content differences stay distinct', () => {
  const payload = clipboardPayloadFromStrings('A\r\nB', 'A\nB', [])
  assert.equal(payload.plain_text, 'A\r\nB')
  assert.equal(payload.html_source, 'A\nB')
})

test('empty, unavailable, oversized, and read failures are controlled', async () => {
  const empty = clipboardPayloadFromStrings('', null, [])
  assert.equal(empty.plain_text, '')
  assert.throws(() => clipboardPayloadFromStrings('x'.repeat(MAX_CLIPBOARD_BYTES + 1), null, []), /./)
  await assert.rejects(
    () => readClipboard(),
    (error: unknown) => error instanceof ClipboardReadError && error.code === 'unavailable',
  )
})

test('maps empty, denied, and browser read failures without echoing clipboard data', async () => {
  const originalNavigator = Object.getOwnPropertyDescriptor(globalThis, 'navigator')
  const restoreNavigator = () => {
    if (originalNavigator === undefined) {
      Reflect.deleteProperty(globalThis, 'navigator')
    } else {
      Object.defineProperty(globalThis, 'navigator', originalNavigator)
    }
  }

  try {
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: { clipboard: { read: async () => [] } },
    })
    await assert.rejects(
      () => readClipboard(),
      (error: unknown) => error instanceof ClipboardReadError && error.code === 'empty',
    )

    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: {
        clipboard: {
          read: async () => {
            throw new DOMException('denied', 'NotAllowedError')
          },
        },
      },
    })
    await assert.rejects(
      () => readClipboard(),
      (error: unknown) => error instanceof ClipboardReadError && error.code === 'denied',
    )

    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: {
        clipboard: {
          read: async () => {
            throw new DOMException('read failed', 'DataError')
          },
        },
      },
    })
    await assert.rejects(
      () => readClipboard(),
      (error: unknown) => error instanceof ClipboardReadError && error.code === 'read_failed',
    )
  } finally {
    restoreNavigator()
  }
})
